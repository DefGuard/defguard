import type { QueryKey } from '@tanstack/react-query';
import type { WebErrorCode } from '../../api/types';
import { openModal } from '../../hooks/modalControls/modalsSubjects';
import { ModalName } from '../../hooks/modalControls/modalTypes';
import { m } from '../../paraglide/messages';

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
