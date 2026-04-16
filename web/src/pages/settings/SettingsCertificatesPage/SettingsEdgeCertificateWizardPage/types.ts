export const SettingsEdgeCertificateWizardStep = {
  ExternalUrlSettings: 'externalUrlSettings',
  ExternalUrlSslConfig: 'externalUrlSslConfig',
  Summary: 'summary',
} as const;

export type SettingsEdgeCertificateWizardStepValue =
  (typeof SettingsEdgeCertificateWizardStep)[keyof typeof SettingsEdgeCertificateWizardStep];
