import { Subject } from 'rxjs';
import { createWithEqualityFn } from 'zustand/traditional';

import { Device } from '../../../shared/types';
import { AddDeviceMethod } from '../types';

const defaultValues: StoreValues = {
  nextSubject: new Subject<void>(),
  currentStep: 0,
  method: AddDeviceMethod.DESKTOP,
  userData: undefined,
  loading: false,
  publicKey: undefined,
  privateKey: undefined,
  device: undefined,
};

export const useAddDevicePageStore = createWithEqualityFn<Store>(
  (set, get) => ({
    ...defaultValues,
    nextStep: (values) => {
      const current = get().currentStep;
      if (values) {
        set({ ...values, currentStep: current + 1 });
      } else {
        set({ currentStep: current + 1 });
      }
    },
    reset: () => set(defaultValues),
    init: (userData) => {
      set({ ...defaultValues, userData });
    },
    setState: (values) => set({ ...values }),
  }),
  Object.is,
);

type Store = StoreValues & StoreMethods;

type StoreValues = {
  loading: boolean;
  nextSubject: Subject<void>;
  currentStep: number;
  method: AddDeviceMethod;
  privateKey?: string;
  publicKey?: string;
  device?: Device;
  userData?: {
    id: number;
    username: string;
    reservedDevices: string[];
  };
};

type StoreMethods = {
  nextStep: (values?: Partial<StoreValues>) => void;
  init: (userData: StoreValues['userData']) => void;
  reset: () => void;
  setState: (values: Partial<StoreValues>) => void;
};
