import { omit } from 'lodash-es';
import { Subject } from 'rxjs';
import { create } from 'zustand';
import { createJSONStorage, persist } from 'zustand/middleware';

export const useWizardStore = create<WizardStore>()(
  persist(
    (set, get) => ({
      currentStep: 0,
      setupType: undefined,
      formSubmitSubject: new Subject<void>(),
      setState: (newState) => set((old) => ({ ...old, ...newState })),
      nextStep: () => set({ currentStep: get().currentStep + 1 }),
    }),
    {
      name: 'network-wizard',
      partialize: (store) => omit(store, ['setState', 'nextStep']),
      storage: createJSONStorage(() => sessionStorage),
    }
  )
);

export enum WizardSetupType {
  'IMPORT' = 'IMPORT',
  'MANUAL' = 'MANUAL',
}

export type WizardStore = {
  currentStep: number;
  formSubmitSubject: Subject<void>;
  setupType?: WizardSetupType;
  importNetworkConfig?: {
    name: string;
    endpoint: string;
    config: string;
  };
  manualNetworkConfig?: {
    name: string;
    address: string;
    port: number;
    endpoint: string;
    allowed_ips?: string;
    dns?: string;
  };
  setState: (newState: Partial<WizardStore>) => void;
  nextStep: () => void;
};
