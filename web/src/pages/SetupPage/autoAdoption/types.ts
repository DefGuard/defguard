export const AutoAdoptionSetupStep = {
  AdminUser: 'adminUser',
  UrlSettings: 'urlSettings',
  VpnSettings: 'vpnSettings',
  MfaSetup: 'mfaSetup',
  Summary: 'summary',
} as const;

export type AutoAdoptionSetupStepValue =
  (typeof AutoAdoptionSetupStep)[keyof typeof AutoAdoptionSetupStep];

export const AutoAdoptionSetupSteps = [
  AutoAdoptionSetupStep.AdminUser,
  AutoAdoptionSetupStep.UrlSettings,
  AutoAdoptionSetupStep.VpnSettings,
  AutoAdoptionSetupStep.MfaSetup,
  AutoAdoptionSetupStep.Summary,
] as const;
