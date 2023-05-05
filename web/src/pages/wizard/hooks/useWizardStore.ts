import { cloneDeep, omit } from 'lodash-es';
import { Subject } from 'rxjs';
import { create } from 'zustand';
import { createJSONStorage, persist } from 'zustand/middleware';

import { ImportedDevice, Network } from '../../../shared/types';

export const useWizardStore = create<WizardStore>()(
  persist(
    (set, get) => ({
      disableBack: false,
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
      setState: (newState) => set((old) => ({ ...old, ...newState })),
      nextStep: () => set({ currentStep: get().currentStep + 1 }),
      perviousStep: () => {
        if (!get().disableBack) {
          return set({ currentStep: get().currentStep - 1 });
        }
      },
      mapDevice: (deviceIP, userId) => {
        const clone = cloneDeep(get().importedNetworkDevices);
        if (clone) {
          const deviceIndex = clone.findIndex((d) => d.wireguard_ip === deviceIP);
          const device = clone[deviceIndex];
          if (device) {
            device.user_id = userId;
            clone[deviceIndex] = device;
            return set({ importedNetworkDevices: clone });
          }
        }
      },
      resetState: () =>
        set({
          disableBack: false,
          loading: false,
          currentStep: 0,
          setupType: WizardSetupType.MANUAL,
          importedNetworkDevices: undefined,
        }),
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
          'mapDevice',
        ]),
      storage: createJSONStorage(() => localStorage),
    }
  )
);

export enum WizardSetupType {
  'IMPORT' = 'IMPORT',
  'MANUAL' = 'MANUAL',
}

export type WizardStore = {
  disableBack: boolean;
  currentStep: number;
  submitSubject: Subject<void>;
  nextStepSubject: Subject<void>;
  loading: boolean;
  setupType?: WizardSetupType;
  importedNetworkConfig?: Network;
  manualNetworkConfig: {
    name: string;
    address: string;
    port: number;
    endpoint: string;
    allowed_ips: string;
    dns?: string;
  };
  importedNetworkDevices?: ImportedDevice[];
  setState: (newState: Partial<WizardStore>) => void;
  resetState: () => void;
  nextStep: () => void;
  perviousStep: () => void;
  mapDevice: (deviceIP: string, userId: number) => void;
};
