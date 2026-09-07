import { m } from '../../paraglide/messages';
import { MfaFlowMethod, type MfaFlowMethodValue } from '../api/types';

export const mfaFlowMethodLabels: Record<MfaFlowMethodValue, string> = {
  [MfaFlowMethod.MobileApprove]: m.mfa_flow_method_mobile_client(),
  [MfaFlowMethod.Totp]: m.mfa_flow_method_authenticator_app(),
  [MfaFlowMethod.OpenId]: m.mfa_flow_method_external_provider(),
  [MfaFlowMethod.Email]: m.mfa_flow_method_email_code(),
  [MfaFlowMethod.Biometric]: m.mfa_flow_method_biometric(),
};
