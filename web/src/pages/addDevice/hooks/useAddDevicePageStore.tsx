import { omit } from 'lodash-es';
import { Subject } from 'rxjs';
import { createJSONStorage, persist } from 'zustand/middleware';
import { createWithEqualityFn } from 'zustand/traditional';

import { DeviceConfigsCardNetworkInfo } from '../../../shared/components/network/DeviceConfigsCard/types';
import { AddDeviceResponseDevice } from '../../../shared/types';
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
  networks: undefined,
  enrollment: undefined,
};

export const useAddDevicePageStore = createWithEqualityFn<Store>()(
  persist(
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
    {
      name: 'add-device-store',
      partialize: (store) => omit(store, ['nextSubject', 'loading']),
      storage: createJSONStorage(() => sessionStorage),
    },
  ),
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
  device?: AddDeviceResponseDevice;
  networks?: DeviceConfigsCardNetworkInfo[];
  userData?: {
    id: number;
    username: string;
    reservedDevices: string[];
    email: string;
  };
  enrollment?: {
    token: string;
    url: string;
  };
};

type StoreMethods = {
  nextStep: (values?: Partial<StoreValues>) => void;
  init: (userData: StoreValues['userData']) => void;
  reset: () => void;
  setState: (values: Partial<StoreValues>) => void;
};
