export type {
  CertInfo,
  ExternalSslType,
  InternalSslType,
} from '../../../shared/api/types';

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
