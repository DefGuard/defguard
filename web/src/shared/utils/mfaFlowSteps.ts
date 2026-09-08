import { m } from '../../paraglide/messages';
import {
  MfaFlowMethod,
  type MfaFlowMethodValue,
  MfaMethodAvailabilityReason,
  type MfaMethodAvailabilityReasonValue,
} from '../api/types';

export const mfaFlowMethodLabels: Record<MfaFlowMethodValue, string> = {
  [MfaFlowMethod.MobileApprove]: m.mfa_flow_method_mobile_client(),
  [MfaFlowMethod.Totp]: m.mfa_flow_method_authenticator_app(),
  [MfaFlowMethod.OpenId]: m.mfa_flow_method_external_provider(),
  [MfaFlowMethod.Email]: m.mfa_flow_method_email_code(),
  [MfaFlowMethod.Biometric]: m.mfa_flow_method_biometric(),
};

export const mfaFlowUnavailableText = (
  reason: MfaMethodAvailabilityReasonValue | null,
): string | undefined => {
  switch (reason) {
    case MfaMethodAvailabilityReason.Licensed:
      return m.location_mfa_flow_unavailable_license();
    case MfaMethodAvailabilityReason.SmtpNotConfigured:
      return m.location_mfa_flow_unavailable_smtp();
    case MfaMethodAvailabilityReason.OidcProviderMissing:
      return m.location_mfa_flow_unavailable_oidc();
    default:
      return undefined;
  }
};
