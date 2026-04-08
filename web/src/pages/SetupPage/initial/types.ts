export const SetupPageStep = {
  // Welcome: 'welcome',
  AdminUser: 'adminUser',
  GeneralConfig: 'generalConfig',
  InternalUrlSettings: 'internalUrlSettings',
  InternalUrlSslConfig: 'internalUrlSslConfig',
  ExternalUrlSettings: 'externalUrlSettings',
  ExternalUrlSslConfig: 'externalUrlSslConfig',
  CertificateAuthority: 'certificateAuthority',
  CASummary: 'certificateAuthoritySummary',
  EdgeDeploy: 'edgeDeploy',
  EdgeComponent: 'edgeComponent',
  EdgeAdoption: 'edgeAdoption',
  Confirmation: 'confirmation',
} as const;

export type SetupPageStepValue = (typeof SetupPageStep)[keyof typeof SetupPageStep];
