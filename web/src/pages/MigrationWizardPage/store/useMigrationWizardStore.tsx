import { omit } from 'lodash-es';
import { create } from 'zustand';
import { createJSONStorage, persist } from 'zustand/middleware';
import type { EdgeAdoptionState } from '../../EdgeSetupPage/types';
import {
  type CAOptionType,
  MigrationWizardStep,
  type MigrationWizardStepValue,
} from '../types';

interface StoreValues {
  isWelcome: boolean;
  activeStep: MigrationWizardStepValue;
  ca_common_name: string;
  ca_email: string;
  ca_validity_period_years: number;
  ca_cert_file: File | null;
  ca_option: CAOptionType | null;
  common_name: string;
  ip_or_domain: string;
  grpc_port: number;
  edgeAdoptionState: EdgeAdoptionState;
}

const edgeAdoptionStateDefaults: EdgeAdoptionState = {
  isProcessing: false,
  isComplete: false,
  currentStep: null,
  errorMessage: null,
  proxyVersion: null,
  proxyLogs: [],
};

const defaults: StoreValues = {
  isWelcome: true,
  activeStep: MigrationWizardStep.General,
  ca_common_name: '',
  ca_email: '',
  ca_validity_period_years: 5,
  ca_cert_file: null,
  ca_option: null,
  common_name: '',
  ip_or_domain: '',
  grpc_port: 50051,
  edgeAdoptionState: edgeAdoptionStateDefaults,
};

interface Store extends StoreValues {
  setActiveStep: (step: MigrationWizardStepValue) => void;
  setState: (values: Partial<StoreValues>) => void;
  resetEdgeAdoptionState: () => void;
  setEdgeAdoptionState: (state: Partial<EdgeAdoptionState>) => void;
}

export const useMigrationWizardStore = create<Store>()(
  persist(
    (set) => ({
      ...defaults,
      setActiveStep: (step) => set({ activeStep: step }),
      setState: (newValues) => {
        set(newValues);
      },
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
      name: 'migration-wizard',
      storage: createJSONStorage(() => sessionStorage),
      partialize: (state) =>
        omit(state, [
          'setActiveStep',
          'setState',
          'resetEdgeAdoptionState',
          'setEdgeAdoptionState',
        ]),
    },
  ),
);
