import { omit } from 'lodash-es';
import { create } from 'zustand';
import { createJSONStorage, persist } from 'zustand/middleware';
import type { SetupStepId } from './steps/types';
import { GatewaySetupStep, type GatewaySetupStepValue } from './types';

type GatewayAdaptationState = {
  isProcessing: boolean;
  isComplete: boolean;
  currentStep: SetupStepId | null;
  errorMessage: string | null;
  gatewayVersion: string | null;
  gatewayLogs: string[];
};

type StoreValues = {
  activeStep: GatewaySetupStepValue;
  showWelcome: boolean;
  common_name: string;
  ip_or_domain: string;
  grpc_port: number;
  network_id: number | null;
  gatewayAdaptationState: GatewayAdaptationState;
};

type StoreMethods = {
  reset: () => void;
  start: (values?: Partial<StoreValues>) => void;
  setActiveStep: (step: GatewaySetupStepValue) => void;
  setShowWelcome: (show: boolean) => void;
  updateValues: (values: Partial<StoreValues>) => void;
  resetGatewayAdaptationState: () => void;
  setGatewayAdaptationState: (state: GatewayAdaptationState) => void;
};

const gatewayAdaptationStateDefaults: GatewayAdaptationState = {
  isProcessing: false,
  isComplete: false,
  currentStep: null,
  errorMessage: null,
  gatewayVersion: null,
  gatewayLogs: [],
};

const defaults: StoreValues = {
  activeStep: GatewaySetupStep.GatewayComponent,
  showWelcome: true,
  common_name: '',
  ip_or_domain: '',
  grpc_port: 50066,
  network_id: null,
  gatewayAdaptationState: gatewayAdaptationStateDefaults,
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
      setShowWelcome: (show) => set({ showWelcome: show }),
      updateValues: (values) => set(values),
      resetGatewayAdaptationState: () =>
        set(() => ({
          gatewayAdaptationState: { ...gatewayAdaptationStateDefaults },
        })),
      setGatewayAdaptationState: (state: Partial<GatewayAdaptationState>) =>
        set((s) => ({
          gatewayAdaptationState: { ...s.gatewayAdaptationState, ...state },
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
          'updateValues',
          'setShowWelcome',
          'resetEdgeAdaptationState',
          'setEdgeAdaptationState',
        ]),
    },
  ),
);
