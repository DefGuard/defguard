import { m } from '../../paraglide/messages';
import {
  type MfaFlowListItemResponse,
  MfaFlowMethod,
  type MfaFlowMethodValue,
  MfaMethodAvailabilityReason,
  type MfaMethodAvailabilityReasonValue,
} from '../api/types';
import type { IconKindValue } from '../defguard-ui/components/Icon/icon-types';

export const mfaFlowMethodLabels: Record<MfaFlowMethodValue, string> = {
  [MfaFlowMethod.MobileApprove]: m.mfa_flow_method_mobile_client(),
  [MfaFlowMethod.Totp]: m.mfa_flow_method_authenticator_app(),
  [MfaFlowMethod.OpenId]: m.mfa_flow_method_external_provider(),
  [MfaFlowMethod.Email]: m.mfa_flow_method_email_code(),
  [MfaFlowMethod.Biometric]: m.mfa_flow_method_biometric(),
  [MfaFlowMethod.Fido2]: m.mfa_flow_method_fido2(),
};

export const mfaFlowMethodHints: Partial<
  Record<
    MfaFlowMethodValue,
    { platform: string; icon: IconKindValue; description: string }
  >
> = {
  [MfaFlowMethod.MobileApprove]: {
    platform: m.mfa_flow_method_desktop_only(),
    icon: 'desktop',
    description: m.mfa_flow_method_mobile_client_description(),
  },
  [MfaFlowMethod.Biometric]: {
    platform: m.mfa_flow_method_mobile_only(),
    icon: 'mobile',
    description: m.mfa_flow_method_biometric_description(),
  },
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

export const openIdProviderDeleteBody = (
  flows: Pick<MfaFlowListItemResponse, 'title' | 'steps'>[],
): string => {
  const usingOidc = flows.filter((flow) =>
    flow.steps.some((step) => step.methods.includes(MfaFlowMethod.OpenId)),
  );

  if (usingOidc.length === 0) return m.settings_openid_provider_delete_confirm_body();

  return m.settings_openid_provider_delete_confirm_body_mfa({
    flows: usingOidc.map((flow) => flow.title).join(', '),
  });
};
