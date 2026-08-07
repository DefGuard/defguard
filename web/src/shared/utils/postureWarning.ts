import { m } from '../../paraglide/messages';
import { openModal } from '../hooks/modalControls/modalsSubjects';
import { ModalName } from '../hooks/modalControls/modalTypes';

type Option = { readonly id: number; readonly label: string };

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
 * One bold heading followed by the ids as label bullets, sorted by label, or
 * `null` when there are no ids so the group is omitted along with its heading.
 */
const formatGroup = (
  heading: string,
  ids: number[],
  labelFor: (id: number) => string,
) => {
  if (ids.length === 0) return null;

  const bullets = ids
    .map(labelFor)
    .sort((left, right) => left.localeCompare(right))
    .map((label) => `- ${label}`)
    .join('\n');

  return `**${heading}**\n\n${bullets}`;
};

/**
 * Diffs id sets, resolves labels, composes the four states (diff only,
 * deferred-enforcement only, both, neither) and opens the ConfirmAction
 * modal. Returns `true` when a modal was opened.
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

  const labelMap = new Map(options.map((option) => [option.id, option.label]));
  const labelFor = (id: number) => labelMap.get(id) ?? String(id);

  const parts = [
    formatGroup(m.modal_posture_assignment_warning_added(), addedIds, labelFor),
    formatGroup(m.modal_posture_assignment_warning_removed(), removedIds, labelFor),
  ].filter((part): part is string => part !== null);

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
 * Warn before changing which posture checks apply to a location. `current` and
 * `next` hold posture-check ids; the body warns about active sessions on this
 * location.
 *
 * Returns `true` when a modal was opened, which is not the same as the admin
 * agreeing: the modal is fired and forgotten, and `actionPromise` runs only if
 * they confirm. A `false` return means there was nothing to warn about, so the
 * caller should proceed with its own save.
 */
export const confirmLocationPostureChange = (args: SelectionChangeArgs): boolean =>
  confirmSelectionChange({
    ...args,
    bodyMessage: m.modal_posture_assignment_warning_body_location,
  });

/**
 * Warn before changing which locations a posture check applies to. `current` and
 * `next` hold location ids. When `deferredEnforcement` is true, appends the
 * rules-deferred paragraph to the warning body.
 *
 * Returns `true` when a modal was opened; see
 * {@link confirmLocationPostureChange} for what that does and does not mean.
 */
export const confirmPostureLocationChange = (
  args: SelectionChangeArgs & { deferredEnforcement?: boolean },
): boolean =>
  confirmSelectionChange({
    ...args,
    bodyMessage: m.modal_posture_assignment_warning_body_postures,
  });
