import type { SetupStepId } from './steps/types';

export const EdgeSetupStep = {
  EdgeComponent: 'edgeComponent',
  EdgeAdaptation: 'edgeAdaptation',
  Confirmation: 'confirmation',
} as const;

export type EdgeAdaptationState = {
  isProcessing: boolean;
  isComplete: boolean;
  currentStep: SetupStepId | null;
  errorMessage: string | null;
  proxyVersion: string | null;
  proxyLogs: string[];
};

export type EdgeSetupStepValue = (typeof EdgeSetupStep)[keyof typeof EdgeSetupStep];
