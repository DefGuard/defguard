import { m } from '../../paraglide/messages';
import { openModal } from '../hooks/modalControls/modalsSubjects';
import { ModalName } from '../hooks/modalControls/modalTypes';

type Option = { readonly id: number; readonly label: string };

/** Escape markdown control characters so admin-supplied labels render as literal text. */
const escapeMarkdown = (s: string) => s.replace(/[\\`*_{}[\]()#+\-.!>|~]/g, '\\$&');

type SelectionChangeArgs = {
  current: Iterable<number>;
  next: Iterable<number>;
  options: readonly Option[];
  actionPromise: () => Promise<unknown>;
};

type ConfirmSelectionChangeArgs = SelectionChangeArgs & {
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
export const confirmPostureSelectionChange = (args: SelectionChangeArgs): boolean =>
  confirmSelectionChange({
    ...args,
    bodyMessage: m.modal_posture_assignment_warning_body_location,
  });

/**
 * Warn when the set of assigned locations on a posture check changes.
 * When `deferredEnforcement` is true, appends the rules-deferred paragraph
 * to the warning body.
 */
export const confirmLocationSelectionChange = (
  args: SelectionChangeArgs & { deferredEnforcement?: boolean },
): boolean =>
  confirmSelectionChange({
    ...args,
    bodyMessage: m.modal_posture_assignment_warning_body_postures,
  });
