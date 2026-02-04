export const SetupPageStep = {
  // Welcome: 'welcome',
  AdminUser: 'adminUser',
  GeneralConfig: 'generalConfig',
  CertificateAuthority: 'certificateAuthority',
  CASummary: 'certificateAuthoritySummary',
  EdgeComponent: 'edgeComponent',
  EdgeAdoption: 'edgeAdoption',
  Confirmation: 'confirmation',
} as const;

export const CAOption = {
  Create: 'create',
  UseOwn: 'useOwn',
} as const;

export type SetupPageStepValue = (typeof SetupPageStep)[keyof typeof SetupPageStep];

export type CAOptionType = (typeof CAOption)[keyof typeof CAOption];
