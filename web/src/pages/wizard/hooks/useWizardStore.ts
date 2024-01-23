import { omit } from 'lodash-es';
import { Subject } from 'rxjs';
import { createJSONStorage, persist } from 'zustand/middleware';
import { createWithEqualityFn } from 'zustand/traditional';

import { ImportedDevice, Network } from '../../../shared/types';

export enum WizardSetupType {
  'IMPORT' = 'IMPORT',
  'MANUAL' = 'MANUAL',
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
    mfa_enabled: false,
    keepalive_interval: 25,
    peer_disconnect_threshold: 75,
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
    mfa_enabled: boolean;
    keepalive_interval: number;
    peer_disconnect_threshold: number;
  };
};

type StoreMethods = {
  setImportedDevices: (devices: ImportedDevice[]) => void;
  setState: (newState: Partial<WizardStore>) => void;
  resetState: () => void;
  nextStep: () => void;
  perviousStep: () => void;
};
