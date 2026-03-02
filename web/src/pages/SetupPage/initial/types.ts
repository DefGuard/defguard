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

export const SetupWizardFlow = {
  Initial: 'initial',
  AutoAdoption: 'postAuto',
} as const;

export const CAOption = {
  Create: 'create',
  UseOwn: 'useOwn',
} as const;

export type SetupPageStepValue = (typeof SetupPageStep)[keyof typeof SetupPageStep];

export type SetupWizardFlowValue = (typeof SetupWizardFlow)[keyof typeof SetupWizardFlow];

export const DefaultSetupSteps = [
  SetupPageStep.AdminUser,
  SetupPageStep.GeneralConfig,
  SetupPageStep.CertificateAuthority,
  SetupPageStep.CASummary,
  SetupPageStep.EdgeComponent,
  SetupPageStep.EdgeAdoption,
  SetupPageStep.Confirmation,
] as const;

export type CAOptionType = (typeof CAOption)[keyof typeof CAOption];
