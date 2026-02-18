import type { SetupStepId } from './steps/types';

export const EdgeSetupStep = {
  EdgeDeploy: 'edgeDeploy',
  EdgeComponent: 'edgeComponent',
  EdgeAdoption: 'edgeAdoption',
  Confirmation: 'confirmation',
} as const;

export type EdgeAdoptionState = {
  isProcessing: boolean;
  isComplete: boolean;
  currentStep: SetupStepId | null;
  errorMessage: string | null;
  proxyVersion: string | null;
  proxyLogs: string[];
};

export type EdgeSetupStepValue = (typeof EdgeSetupStep)[keyof typeof EdgeSetupStep];
