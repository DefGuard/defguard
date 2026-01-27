import { omit } from 'lodash-es';
import { create } from 'zustand';
import { createJSONStorage, persist } from 'zustand/middleware';
import type { SetupStepId } from './steps/types';
import { EdgeSetupStep, type EdgeSetupStepValue } from './types';

type EdgeAdaptationState = {
  isProcessing: boolean;
  isComplete: boolean;
  currentStep: SetupStepId | null;
  errorMessage: string | null;
  proxyVersion: string | null;
  proxyLogs: string[];
};

type StoreValues = {
  activeStep: EdgeSetupStepValue;
  showWelcome: boolean;
  common_name: string;
  ip_or_domain: string;
  grpc_port: number;
  public_domain: string;
  edgeAdaptationState: EdgeAdaptationState;
};

type StoreMethods = {
  reset: () => void;
  start: (values?: Partial<StoreValues>) => void;
  setActiveStep: (step: EdgeSetupStepValue) => void;
  setShowWelcome: (show: boolean) => void;
  updateValues: (values: Partial<StoreValues>) => void;
  resetEdgeAdaptationState: () => void;
  setEdgeAdaptationState: (state: EdgeAdaptationState) => void;
};

const edgeAdaptationStateDefaults: EdgeAdaptationState = {
  isProcessing: false,
  isComplete: false,
  currentStep: null,
  errorMessage: null,
  proxyVersion: null,
  proxyLogs: [],
};

const defaults: StoreValues = {
  activeStep: EdgeSetupStep.EdgeComponent,
  showWelcome: true,
  common_name: '',
  ip_or_domain: '',
  grpc_port: 50051,
  public_domain: '',
  edgeAdaptationState: edgeAdaptationStateDefaults,
};

export const useEdgeWizardStore = create<StoreMethods & StoreValues>()(
  persist(
    (set) => ({
      ...defaults,
      reset: () => set(defaults),
      start: (initial) => {
        set({
          ...defaults,
          ...initial,
        });
      },
      setActiveStep: (step) => set({ activeStep: step }),
      setShowWelcome: (show) => set({ showWelcome: show }),
      updateValues: (values) => set(values),
      resetEdgeAdaptationState: () =>
        set(() => ({
          edgeAdaptationState: { ...edgeAdaptationStateDefaults },
        })),
      setEdgeAdaptationState: (state: Partial<EdgeAdaptationState>) =>
        set((s) => ({
          edgeAdaptationState: { ...s.edgeAdaptationState, ...state },
        })),
    }),
    {
      name: 'setup-wizard-store',
      storage: createJSONStorage(() => sessionStorage),
      partialize: (state) =>
        omit(state, [
          'reset',
          'start',
          'setActiveStep',
          'updateValues',
          'setShowWelcome',
          'resetEdgeAdaptationState',
          'setEdgeAdaptationState',
        ]),
    },
  ),
);
