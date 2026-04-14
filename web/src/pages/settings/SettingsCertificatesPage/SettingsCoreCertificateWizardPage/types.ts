export const SettingsCoreCertificateWizardStep = {
  InternalUrlSettings: 'internalUrlSettings',
  InternalUrlSslConfig: 'internalUrlSslConfig',
  Summary: 'summary',
} as const;

export type SettingsCoreCertificateWizardStepValue =
  (typeof SettingsCoreCertificateWizardStep)[keyof typeof SettingsCoreCertificateWizardStep];
