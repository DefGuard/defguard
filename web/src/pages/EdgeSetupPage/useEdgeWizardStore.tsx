import { omit } from 'lodash-es';
import { create } from 'zustand';
import { createJSONStorage, persist } from 'zustand/middleware';
import { type EdgeAdoptionState, EdgeSetupStep, type EdgeSetupStepValue } from './types';

type StoreValues = {
  activeStep: EdgeSetupStepValue;
  isOnWelcomePage: boolean;
  common_name: string;
  ip_or_domain: string;
  grpc_port: number;
  edgeAdoptionState: EdgeAdoptionState;
};

type StoreMethods = {
  reset: () => void;
  start: (values?: Partial<StoreValues>) => void;
  setActiveStep: (step: EdgeSetupStepValue) => void;
  setisOnWelcomePage: (show: boolean) => void;
  resetEdgeAdoptionState: () => void;
  setEdgeAdoptionState: (state: EdgeAdoptionState) => void;
};

const edgeAdoptionStateDefaults: EdgeAdoptionState = {
  isProcessing: false,
  isComplete: false,
  currentStep: null,
  errorMessage: null,
  proxyVersion: null,
  proxyLogs: [],
};

const defaults: StoreValues = {
  activeStep: EdgeSetupStep.EdgeDeploy,
  isOnWelcomePage: true,
  common_name: '',
  ip_or_domain: '',
  grpc_port: 50051,
  edgeAdoptionState: edgeAdoptionStateDefaults,
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
      setisOnWelcomePage: (show) => set({ isOnWelcomePage: show }),
      resetEdgeAdoptionState: () =>
        set(() => ({
          edgeAdoptionState: { ...edgeAdoptionStateDefaults },
        })),
      setEdgeAdoptionState: (state: Partial<EdgeAdoptionState>) =>
        set((s) => ({
          edgeAdoptionState: { ...s.edgeAdoptionState, ...state },
        })),
    }),
    {
      name: 'edge-wizard-store',
      storage: createJSONStorage(() => sessionStorage),
      partialize: (state) =>
        omit(state, [
          'reset',
          'start',
          'setActiveStep',
          'setisOnWelcomePage',
          'resetEdgeAdoptionState',
          'setEdgeAdoptionState',
        ]),
    },
  ),
);
