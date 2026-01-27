export const EdgeSetupStep = {
  EdgeComponent: 'edgeComponent',
  EdgeAdaptation: 'edgeAdaptation',
  Confirmation: 'confirmation',
} as const;

export type EdgeSetupStepValue = (typeof EdgeSetupStep)[keyof typeof EdgeSetupStep];
