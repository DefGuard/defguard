import { sentenceCase } from 'text-case';
import { m } from '../../paraglide/messages';
import {
  ActivityLogEventType,
  type ActivityLogEventTypeValue,
} from '../../shared/api/activity-log-types';
import type { ActivityLogEventMetadata } from '../../shared/api/types';
import { isPresent } from '../../shared/defguard-ui/utils/isPresent';

/**
 * Events whose details are presented in the log details drawer. Mirrors the settings
 * variants of `ApiEventType` that carry a `before`/`after` payload, plus
 * `SettingsDefaultBrandingRestored`, which records no payload but is still a settings change.
 */
const detailedLogEvents = new Set<ActivityLogEventTypeValue>([
  ActivityLogEventType.SettingsUpdated,
  ActivityLogEventType.SettingsUpdatedPartial,
  ActivityLogEventType.SettingsDefaultBrandingRestored,
  ActivityLogEventType.EnterpriseSettingsUpdated,
]);

export const hasLogDetails = (event: ActivityLogEventTypeValue): boolean =>
  detailedLogEvents.has(event);

/**
 * Field labels follow the `activity_log_field_<field>` message convention, the same way
 * event labels do in `activity-log-types.ts`. Fields with no message fall back to a
 * humanized field name, so settings added on the backend still render sensibly.
 */
// The message bundle is keyed by literal names, so a runtime-built key needs the cast.
const fieldMessages = m as unknown as Record<string, (() => string) | undefined>;

const fieldLabel = (field: string): string =>
  fieldMessages[`activity_log_field_${field}`]?.() ?? sentenceCase(field);

export const missingValuePlaceholder = '—';
/** Stand-in for a credential's value, which the backend never records. */
const protectedValuePlaceholder = '••••••';

const formatValue = (value: unknown): string => {
  if (!isPresent(value)) return missingValuePlaceholder;
  if (typeof value === 'boolean') return value ? m.state_enabled() : m.state_disabled();
  if (typeof value === 'number') return String(value);
  if (typeof value === 'string') {
    return value.length > 0 ? value : missingValuePlaceholder;
  }
  if (Array.isArray(value)) {
    return value.length > 0
      ? value.map((item) => formatValue(item)).join(', ')
      : missingValuePlaceholder;
  }
  return JSON.stringify(value);
};

type LogDetailsChange = {
  field: string;
  label: string;
  from: string;
  to: string;
  changed: boolean;
};

const asRecord = (value: unknown): Record<string, unknown> | null => {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) return null;
  return value as Record<string, unknown>;
};

type ProtectedFieldState = {
  changed: boolean;
  wasSet: boolean;
  isSet: boolean;
};

/**
 * Credential state by field name. `before`/`after` anonymize these to "is it set" booleans,
 * so whether one actually changed can only come from here — a rotated password reads as set
 * on both sides.
 */
const readProtectedFields = (
  metadata: ActivityLogEventMetadata,
): Map<string, ProtectedFieldState> => {
  const entries = metadata.protected_fields;
  if (!Array.isArray(entries)) return new Map();

  const states = new Map<string, ProtectedFieldState>();
  for (const entry of entries) {
    const record = asRecord(entry);
    if (!isPresent(record) || typeof record.field !== 'string') continue;
    states.set(record.field, {
      changed: record.changed === true,
      wasSet: record.was_set === true,
      isSet: record.is_set === true,
    });
  }
  return states;
};

/**
 * Turns the `before`/`after` state stored in the event metadata into display ready rows,
 * in the order the backend serialized them. Credentials keep their place among the settings
 * they belong to, showing a mask instead of a value.
 */
export const buildLogDetailsChanges = (
  metadata: ActivityLogEventMetadata | null,
): LogDetailsChange[] => {
  if (!isPresent(metadata)) return [];
  const before = asRecord(metadata.before);
  const after = asRecord(metadata.after);
  if (!isPresent(before) || !isPresent(after)) return [];

  const protectedFields = readProtectedFields(metadata);
  // A field is listed once, even when both the snapshots and the credential list name it.
  const fields = [
    ...new Set([
      ...Object.keys(before),
      ...Object.keys(after),
      ...protectedFields.keys(),
    ]),
  ];

  return fields.map((field) => {
    const label = fieldLabel(field);
    const state = protectedFields.get(field);

    if (isPresent(state)) {
      return {
        field,
        label,
        from: state.wasSet ? protectedValuePlaceholder : missingValuePlaceholder,
        to: state.isSet ? protectedValuePlaceholder : missingValuePlaceholder,
        changed: state.changed,
      };
    }

    const from = formatValue(before[field]);
    const to = formatValue(after[field]);
    return { field, label, from, to, changed: from !== to };
  });
};
