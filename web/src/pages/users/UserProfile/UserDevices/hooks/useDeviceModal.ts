import { createWithEqualityFn } from 'zustand/traditional';

import { AddDeviceConfig, Device, StandardModalState } from '../../../../../shared/types';

export enum DeviceModalSetupMode {
  AUTO_CONFIG = 1,
  MANUAL_CONFIG = 2,
}

const defaultValues: StoreValues = {
  visible: false,
  currentStep: 0,
  endStep: 1,
  setupMode: DeviceModalSetupMode.AUTO_CONFIG,
  configs: undefined,
  deviceName: undefined,
  device: undefined,
};

export const useDeviceModal = createWithEqualityFn<Store>(
  (set, get) => ({
    ...defaultValues,
    nextStep: (values) => {
      const { currentStep, endStep } = get();
      // close modal when finished
      if (endStep === currentStep) {
        return set({ visible: false });
      } else {
        if (values) {
          return set({ ...values, currentStep: currentStep + 1 });
        } else {
          return set({ currentStep: currentStep + 1 });
        }
      }
    },
    setState: (newValues) => set((old) => ({ ...old, ...newValues })),
    open: (initial) => set({ ...defaultValues, ...initial }),
    reset: () => set(defaultValues),
  }),
  Object.is,
);

type Store = StoreValues & StoreMethods;

type StoreValues = StandardModalState & {
  currentStep: number;
  endStep: number;
  setupMode: DeviceModalSetupMode;
  configs?: AddDeviceConfig[];
  deviceName?: string;
  device?: Device;
};

type StoreMethods = {
  setState: (values: Partial<StoreValues>) => void;
  nextStep: (values?: Partial<StoreValues>) => void;
  open: (values?: Partial<StoreValues>) => void;
  reset: () => void;
};
