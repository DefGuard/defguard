export const WizardCover = {
  Migration: 'migration',
  Location: 'location',
  Edge: 'edge',
  Gateway: 'gateway',
} as const;

export type WizardCoverValue = (typeof WizardCover)[keyof typeof WizardCover];
