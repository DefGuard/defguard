import type { QueryKey } from '@tanstack/react-query';
import { m } from '../../paraglide/messages';
import type { WebErrorCode } from '../api/types';
import { openModal } from '../hooks/modalControls/modalsSubjects';
import { ModalName } from '../hooks/modalControls/modalTypes';

type PostureWarningKind = 'location' | 'postures';

type BuildPostureWarningChangesArgs = {
  added: string[];
  removed: string[];
};

/**
 * Builds the `{changes}` markdown fragment listing added and removed items
 * in two labelled groups, sorted alphabetically. Returns `null` when both
 * sets are empty (no modal needed).
 */
export const buildPostureWarningChanges = ({
  added,
  removed,
}: BuildPostureWarningChangesArgs): string | null => {
  const sortedAdded = [...added].sort((a, b) => a.localeCompare(b));
  const sortedRemoved = [...removed].sort((a, b) => a.localeCompare(b));

  const parts: string[] = [];

  if (sortedAdded.length > 0) {
    parts.push(
      `**${m.modal_posture_assignment_warning_added()}**\n${sortedAdded
        .map((name) => `- ${name}`)
        .join('\n')}`,
    );
  }

  if (sortedRemoved.length > 0) {
    parts.push(
      `**${m.modal_posture_assignment_warning_removed()}**\n${sortedRemoved
        .map((name) => `- ${name}`)
        .join('\n')}`,
    );
  }

  if (parts.length === 0) return null;
  return parts.join('\n\n');
};

type OpenPostureAssignmentWarningArgs = {
  kind: PostureWarningKind;
  added: string[];
  removed: string[];
  extraBody?: string;
  actionPromise: () => Promise<unknown>;
  invalidateKeys?: QueryKey[];
  onSuccess?: (result: unknown) => void;
  onError?: (message: string, code?: WebErrorCode) => void;
};

/**
 * Opens a ConfirmAction modal warning about posture assignment changes.
 * Does nothing when both added and removed sets are empty.
 */
export const openPostureAssignmentWarning = ({
  kind,
  added,
  removed,
  extraBody,
  actionPromise,
  invalidateKeys,
  onSuccess,
  onError,
}: OpenPostureAssignmentWarningArgs) => {
  const changes = buildPostureWarningChanges({ added, removed });

  if (changes === null) return;

  const bodyKey =
    kind === 'location'
      ? m.modal_posture_assignment_warning_body_location({ changes })
      : m.modal_posture_assignment_warning_body_postures({ changes });

  const contentMd = extraBody ? `${bodyKey}\n\n${extraBody}` : bodyKey;

  openModal(ModalName.ConfirmAction, {
    title: m.modal_posture_assignment_warning_title(),
    contentMd,
    actionPromise,
    invalidateKeys,
    submitProps: {
      text: m.controls_save_changes_anyway(),
      variant: 'critical',
    },
    onSuccess,
    onError,
  });
};

type Option = { readonly id: number; readonly label: string };

/** Escape markdown control characters so admin-supplied labels render as literal text. */
const escapeMarkdown = (s: string) => s.replace(/[\\`*_{}[\]()#+\-.!>|~]/g, '\\$&');

type ConfirmSelectionChangeArgs = {
  current: Iterable<number>;
  next: Iterable<number>;
  options: readonly Option[];
  actionPromise: () => Promise<unknown>;
  deferredEnforcement?: boolean;
  bodyMessage: (args: { changes: string }) => string;
};

/**
 * Diffs id sets, resolves labels, escapes markdown, composes the four
 * states (diff only, deferred-enforcement only, both, neither) and opens
 * the ConfirmAction modal. Returns `true` when a modal was opened.
 */
const confirmSelectionChange = ({
  current,
  next,
  options,
  actionPromise,
  deferredEnforcement,
  bodyMessage,
}: ConfirmSelectionChangeArgs): boolean => {
  const currentSet = new Set(current);
  const nextSet = new Set(next);

  const addedIds = [...nextSet].filter((id) => !currentSet.has(id));
  const removedIds = [...currentSet].filter((id) => !nextSet.has(id));

  const labelMap = new Map(options.map((o) => [o.id, o.label]));

  const sortedAdded = addedIds
    .map((id) => escapeMarkdown(labelMap.get(id) ?? String(id)))
    .sort((a, b) => a.localeCompare(b));
  const sortedRemoved = removedIds
    .map((id) => escapeMarkdown(labelMap.get(id) ?? String(id)))
    .sort((a, b) => a.localeCompare(b));

  const parts: string[] = [];

  if (sortedAdded.length > 0) {
    parts.push(
      `**${m.modal_posture_assignment_warning_added()}**\n\n${sortedAdded
        .map((name) => `- ${name}`)
        .join('\n')}`,
    );
  }

  if (sortedRemoved.length > 0) {
    parts.push(
      `**${m.modal_posture_assignment_warning_removed()}**\n\n${sortedRemoved
        .map((name) => `- ${name}`)
        .join('\n')}`,
    );
  }

  const changes = parts.length > 0 ? parts.join('\n\n') : null;
  const hasDiff = changes !== null;

  if (!hasDiff && !deferredEnforcement) return false;

  let contentMd: string;
  if (hasDiff) {
    contentMd = deferredEnforcement
      ? `${bodyMessage({ changes })}\n\n${m.modal_posture_rules_warning_body()}`
      : bodyMessage({ changes });
  } else {
    contentMd = m.modal_posture_rules_warning_body();
  }

  openModal(ModalName.ConfirmAction, {
    title: m.modal_posture_assignment_warning_title(),
    contentMd,
    actionPromise,
    submitProps: {
      text: m.controls_save_changes_anyway(),
      variant: 'critical',
    },
  });

  return true;
};

/**
 * Warn when the set of assigned posture checks on a location changes.
 * Ids are posture-check ids; the body warns about active sessions for
 * this location.
 */
export const confirmPostureSelectionChange = (
  current: Iterable<number>,
  next: Iterable<number>,
  options: readonly Option[],
  actionPromise: () => Promise<unknown>,
): boolean =>
  confirmSelectionChange({
    current,
    next,
    options,
    actionPromise,
    bodyMessage: m.modal_posture_assignment_warning_body_location,
  });

/**
 * Warn when the set of assigned locations on a posture check changes.
 * When `deferredEnforcement` is true, appends the rules-deferred paragraph
 * to the warning body.
 */
export const confirmLocationSelectionChange = (
  current: Iterable<number>,
  next: Iterable<number>,
  options: readonly Option[],
  actionPromise: () => Promise<unknown>,
  deferredEnforcement?: boolean,
): boolean =>
  confirmSelectionChange({
    current,
    next,
    options,
    actionPromise,
    deferredEnforcement,
    bodyMessage: m.modal_posture_assignment_warning_body_postures,
  });
