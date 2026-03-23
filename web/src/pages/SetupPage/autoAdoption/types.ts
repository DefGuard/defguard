export const AutoAdoptionSetupStep = {
  AdminUser: 'adminUser',
  InternalUrlSettings: 'internalUrlSettings',
  InternalUrlSslConfig: 'internalUrlSslConfig',
  ExternalUrlSettings: 'externalUrlSettings',
  ExternalUrlSslConfig: 'externalUrlSslConfig',
  VpnSettings: 'vpnSettings',
  MfaSetup: 'mfaSetup',
  Summary: 'summary',
} as const;

export type AutoAdoptionSetupStepValue =
  (typeof AutoAdoptionSetupStep)[keyof typeof AutoAdoptionSetupStep];

export const AutoAdoptionSetupSteps = [
  AutoAdoptionSetupStep.AdminUser,
  AutoAdoptionSetupStep.InternalUrlSettings,
  AutoAdoptionSetupStep.InternalUrlSslConfig,
  AutoAdoptionSetupStep.ExternalUrlSettings,
  AutoAdoptionSetupStep.ExternalUrlSslConfig,
  AutoAdoptionSetupStep.VpnSettings,
  AutoAdoptionSetupStep.MfaSetup,
  AutoAdoptionSetupStep.Summary,
] as const;

export type InternalSslType = 'none' | 'defguard_ca' | 'own_cert';

export type ExternalSslType = 'none' | 'lets_encrypt' | 'defguard_ca' | 'own_cert';

export interface CertInfo {
  common_name: string;
  valid_for_days: number;
  not_before: string;
  not_after: string;
}
