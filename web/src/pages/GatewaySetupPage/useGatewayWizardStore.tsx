import { omit } from 'lodash-es';
import { create } from 'zustand';
import { createJSONStorage, persist } from 'zustand/middleware';
import type { SetupStepId } from './steps/types';
import { GatewaySetupStep, type GatewaySetupStepValue } from './types';

type GatewayAdoptionState = {
  isProcessing: boolean;
  isComplete: boolean;
  currentStep: SetupStepId | null;
  errorMessage: string | null;
  gatewayVersion: string | null;
  gatewayLogs: string[];
};

type StoreValues = {
  activeStep: GatewaySetupStepValue;
  isOnWelcomePage: boolean;
  common_name: string;
  ip_or_domain: string;
  grpc_port: number;
  network_id: number | null;
  gatewayAdoptionState: GatewayAdoptionState;
};

type StoreMethods = {
  reset: () => void;
  start: (values?: Partial<StoreValues>) => void;
  setActiveStep: (step: GatewaySetupStepValue) => void;
  setisOnWelcomePage: (show: boolean) => void;
  resetGatewayAdoptionState: () => void;
  setGatewayAdoptionState: (state: GatewayAdoptionState) => void;
};

const gatewayAdoptionStateDefaults: GatewayAdoptionState = {
  isProcessing: false,
  isComplete: false,
  currentStep: null,
  errorMessage: null,
  gatewayVersion: null,
  gatewayLogs: [],
};

const defaults: StoreValues = {
  activeStep: GatewaySetupStep.GatewayComponent,
  isOnWelcomePage: true,
  common_name: '',
  ip_or_domain: '',
  grpc_port: 50066,
  network_id: null,
  gatewayAdoptionState: gatewayAdoptionStateDefaults,
};

export const useGatewayWizardStore = create<StoreMethods & StoreValues>()(
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
      resetGatewayAdoptionState: () =>
        set(() => ({
          gatewayAdoptionState: { ...gatewayAdoptionStateDefaults },
        })),
      setGatewayAdoptionState: (state: Partial<GatewayAdoptionState>) =>
        set((s) => ({
          gatewayAdoptionState: { ...s.gatewayAdoptionState, ...state },
        })),
    }),
    {
      name: 'gateway-wizard-store',
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
