import { isAxiosError } from 'axios';
import { m } from '../../paraglide/messages';
import api from '../../shared/api/api';
import type {
  MfaFlowErrorResponse,
  MfaFlowListItemResponse,
} from '../../shared/api/types';
import { isPresent } from '../../shared/defguard-ui/utils/isPresent';
import { openModal } from '../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../shared/hooks/modalControls/modalTypes';
import type { OpenConfirmActionModal } from '../../shared/hooks/modalControls/types';

export const getMfaFlowDeleteBlockingLocations = (
  flow: Pick<MfaFlowListItemResponse, 'locations'>,
): string[] =>
  flow.locations
    .filter((location) => location.mfa_enabled && location.is_default)
    .map((location) => location.name);

export const getMfaFlowDeleteConflictLocations = (error: unknown): string[] | null => {
  if (!isAxiosError<MfaFlowErrorResponse>(error) || error.response?.status !== 409) {
    return null;
  }
  const locations = error.response.data.fields?.[0]?.locations;
  return isPresent(locations) && locations.length > 0 ? locations : null;
};

export const openMfaFlowDeleteBlockedModal = (locations: string[]) => {
  openModal(ModalName.DeleteAliasDestinationBlocked, {
    title: m.modal_delete_acl_blocked_title(),
    description: m.mfa_flow_delete_blocked_body(),
    rules: locations,
  });
};

export const getDeleteMfaFlowModalData = (
  flow: Pick<MfaFlowListItemResponse, 'id' | 'title'>,
): OpenConfirmActionModal => ({
  title: m.mfa_flow_delete_title(),
  contentMd: m.mfa_flow_delete_body({ name: flow.title }),
  actionPromise: () => api.mfaFlow.delete(flow.id),
  invalidateKeys: [['mfa-flow'], ['location']],
  submitProps: {
    text: m.controls_delete(),
    variant: 'critical',
    testId: 'confirm-delete-mfa-flow',
  },
});
