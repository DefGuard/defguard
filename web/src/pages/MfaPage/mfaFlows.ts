import { isAxiosError } from 'axios';
import { m } from '../../paraglide/messages';
import api from '../../shared/api/api';
import type {
  MfaFlowErrorResponse,
  MfaFlowListItemResponse,
} from '../../shared/api/types';
import type { OpenConfirmActionModal } from '../../shared/hooks/modalControls/types';

export const getMfaFlowDeleteErrorMessage = (error: unknown): string => {
  if (!isAxiosError<MfaFlowErrorResponse>(error)) return m.mfa_flow_delete_failed();

  const field = error.response?.data.fields?.[0];
  const locations = field?.locations?.join(', ');
  if (!field || !locations) return m.mfa_flow_delete_failed();

  switch (field.code) {
    case 'location_requires_flow':
      return m.mfa_flow_delete_location_requires_flow({ locations });
    case 'flow_is_default':
      return m.mfa_flow_delete_flow_is_default({ locations });
    default:
      return m.mfa_flow_delete_failed();
  }
};

export const getDeleteMfaFlowModalData = (
  flow: Pick<MfaFlowListItemResponse, 'id' | 'title'>,
): OpenConfirmActionModal => ({
  title: m.mfa_flow_delete_title(),
  contentMd: m.mfa_flow_delete_body({ name: flow.title }),
  actionPromise: () => api.mfaFlow.delete(flow.id),
  invalidateKeys: [['mfa-flow'], ['location']],
  submitProps: { text: m.controls_delete(), variant: 'critical' },
});
