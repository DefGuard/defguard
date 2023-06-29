import { omit } from 'lodash-es';
import { Subject } from 'rxjs';
import { create } from 'zustand';
import { createJSONStorage, persist } from 'zustand/middleware';

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
    dns: '',
  },
};

export const useWizardStore = create<WizardStore>()(
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
    }
  )
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
    dns?: string;
  };
};

type StoreMethods = {
  setImportedDevices: (devices: ImportedDevice[]) => void;
  setState: (newState: Partial<WizardStore>) => void;
  resetState: () => void;
  nextStep: () => void;
  perviousStep: () => void;
};
