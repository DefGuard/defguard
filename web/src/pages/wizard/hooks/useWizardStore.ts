import { omit } from 'lodash-es';
import { Subject } from 'rxjs';
import { createJSONStorage, persist } from 'zustand/middleware';
import { createWithEqualityFn } from 'zustand/traditional';

import {
  type ImportedDevice,
  LocationMfaMode,
  type Network,
} from '../../../shared/types';

export enum WizardSetupType {
  IMPORT = 'IMPORT',
  MANUAL = 'MANUAL',
}

const defaultValues: StoreFields = {
  loading: false,
  currentStep: 0,
  setupType: WizardSetupType.MANUAL,
  importedNetworkDevices: undefined,
  submitSubject: new Subject<void>(),
  nextStepSubject: new Subject<void>(),
  manualNetworkConfig: {
    address: '',
    endpoint: '',
    name: '',
    port: 50051,
    allowed_ips: '',
    allowed_groups: [],
    dns: '',
    keepalive_interval: 25,
    peer_disconnect_threshold: 180,
    acl_enabled: false,
    acl_default_allow: false,
    location_mfa_mode: LocationMfaMode.DISABLED,
  },
};

export const useWizardStore = createWithEqualityFn<WizardStore>()(
  persist(
    (set, get) => ({
      ...defaultValues,
      setState: (newState) => set((old) => ({ ...old, ...newState })),
      nextStep: () => set({ currentStep: get().currentStep + 1 }),
      perviousStep: () => {
        return set({ currentStep: get().currentStep - 1 });
      },
      resetState: () => set(defaultValues),
      setImportedDevices: (devices) => set({ importedNetworkDevices: devices }),
    }),
    {
      name: 'network-wizard',
      partialize: (store) =>
        omit(store, [
          'setState',
          'resetState',
          'nextStep',
          'nextStepSubject',
          'perviousStep',
          'submitSubject',
          'setImportedDevices',
        ]),
      storage: createJSONStorage(() => localStorage),
    },
  ),
  Object.is,
);

export type WizardStore = StoreFields & StoreMethods;

type StoreFields = {
  currentStep: number;
  submitSubject: Subject<void>;
  nextStepSubject: Subject<void>;
  loading: boolean;
  setupType?: WizardSetupType;
  importedNetworkConfig?: Network;
  importedNetworkDevices?: ImportedDevice[];
  manualNetworkConfig: {
    name: string;
    address: string;
    port: number;
    endpoint: string;
    allowed_ips: string;
    allowed_groups: string[];
    dns?: string;
    keepalive_interval: number;
    peer_disconnect_threshold: number;
    acl_enabled: boolean;
    acl_default_allow: boolean;
    location_mfa_mode: LocationMfaMode;
  };
};

type StoreMethods = {
  setImportedDevices: (devices: ImportedDevice[]) => void;
  setState: (newState: Partial<WizardStore>) => void;
  resetState: () => void;
  nextStep: () => void;
  perviousStep: () => void;
};
