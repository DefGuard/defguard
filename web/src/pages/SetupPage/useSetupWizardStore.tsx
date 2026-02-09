import { omit } from 'lodash-es';
import { create } from 'zustand';
import { createJSONStorage, persist } from 'zustand/middleware';
import type { EdgeAdoptionState } from '../EdgeSetupPage/types';
import { type CAOptionType, SetupPageStep, type SetupPageStepValue } from './types';

const edgeAdoptionStateDefaults: EdgeAdoptionState = {
  isProcessing: false,
  isComplete: false,
  currentStep: null,
  errorMessage: null,
  proxyVersion: null,
  proxyLogs: [],
};

type StoreValues = {
  isOnWelcomePage: boolean;
  activeStep: SetupPageStepValue;
  // Admin config
  admin_first_name: string;
  admin_last_name: string;
  admin_username: string;
  admin_email: string;
  admin_password: string;
  // General config
  defguard_url: string;
  default_admin_group_name: string;
  default_authentication_period_days: number;
  default_mfa_code_timeout_seconds: number;
  public_proxy_url: string;
  // CA settings
  ca_common_name: string;
  ca_email: string;
  ca_validity_period_years: number;
  ca_cert_file: File | null;
  ca_option: CAOptionType | null;
  // Edge settings
  common_name: string;
  ip_or_domain: string;
  grpc_port: number;
  edgeAdoptionState: EdgeAdoptionState;
};

type StoreMethods = {
  reset: () => void;
  start: (values?: Partial<StoreValues>) => void;
  setActiveStep: (step: SetupPageStepValue) => void;
  resetEdgeAdoptionState: () => void;
  setEdgeAdoptionState: (state: Partial<EdgeAdoptionState>) => void;
};

const defaults: StoreValues = {
  isOnWelcomePage: true,
  activeStep: SetupPageStep.AdminUser,
  // Admin config
  admin_first_name: '',
  admin_last_name: '',
  admin_username: '',
  admin_email: '',
  admin_password: '',
  // General config
  defguard_url: '',
  default_admin_group_name: 'admin',
  default_authentication_period_days: 30,
  default_mfa_code_timeout_seconds: 300,
  public_proxy_url: '',
  // CA settings
  ca_common_name: '',
  ca_email: '',
  ca_validity_period_years: 5,
  ca_cert_file: null,
  ca_option: null,
  // Edge settings
  common_name: '',
  ip_or_domain: '',
  grpc_port: 50051,
  edgeAdoptionState: edgeAdoptionStateDefaults,
};

export const useSetupWizardStore = create<StoreMethods & StoreValues>()(
  persist(
    (set) => ({
      ...defaults,
      reset: () =>
        set({
          ...defaults,
          isOnWelcomePage: true,
        }),
      start: (initial) => {
        set({
          ...defaults,
          ...initial,
          activeStep: SetupPageStep.AdminUser,
        });
      },
      setActiveStep: (step) => set({ activeStep: step }),
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
      name: 'setup-wizard-store',
      storage: createJSONStorage(() => sessionStorage),
      partialize: (state) =>
        omit(state, [
          'reset',
          'start',
          'setActiveStep',
          'resetEdgeAdoptionState',
          'setEdgeAdoptionState',
        ]),
    },
  ),
);
