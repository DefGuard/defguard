export const MigrationWizardStep = {
  Welcome: 'welcome',
  General: 'general',
  Ca: 'ca',
  CaSummary: 'caSummary',
  EdgeDeployment: 'edgeDeployment',
  Edge: 'edge',
  EdgeAdoption: 'edgeAdoption',
  Confirmation: 'confirmation',
} as const;

export type MigrationWizardStepValue =
  (typeof MigrationWizardStep)[keyof typeof MigrationWizardStep];

export const CAOption = {
  Create: 'create',
  UseOwn: 'useOwn',
} as const;

export type CAOptionType = (typeof CAOption)[keyof typeof CAOption];
