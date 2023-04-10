import { omit } from 'lodash-es';
import { Subject } from 'rxjs';
import { create } from 'zustand';
import { createJSONStorage, persist } from 'zustand/middleware';

import { ImportedDevice, Network } from '../../../shared/types';

export const useWizardStore = create<WizardStore>()(
  persist(
    (set, get) => ({
      disableBack: false,
      disableNext: false,
      currentStep: 0,
      setupType: WizardSetupType.MANUAL,
      importedNetworkDevices: undefined,
      submitSubject: new Subject<void>(),
      setState: (newState) => set((old) => ({ ...old, ...newState })),
      nextStep: () => set({ currentStep: get().currentStep + 1 }),
      perviousStep: () => {
        if (!get().disableBack) {
          return set({ currentStep: get().currentStep - 1 });
        }
      },
    }),
    {
      name: 'network-wizard',
      partialize: (store) =>
        omit(store, ['setState', 'nextStep', 'perviousStep', 'submitSubject']),
      storage: createJSONStorage(() => sessionStorage),
    }
  )
);

export enum WizardSetupType {
  'IMPORT' = 'IMPORT',
  'MANUAL' = 'MANUAL',
}

export type WizardStore = {
  disableBack: boolean;
  disableNext: boolean;
  currentStep: number;
  submitSubject: Subject<void>;
  setupType?: WizardSetupType;
  importedNetworkConfig?: Network;
  manualNetworkConfig?: {
    name: string;
    address: string;
    port: number;
    endpoint: string;
    allowed_ips?: string;
    dns?: string;
  };
  importedNetworkDevices?: ImportedDevice[];
  setState: (newState: Partial<WizardStore>) => void;
  nextStep: () => void;
  perviousStep: () => void;
};
