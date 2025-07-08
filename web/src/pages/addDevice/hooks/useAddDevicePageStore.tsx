import { omit } from 'lodash-es';
import { Subject } from 'rxjs';
import { createJSONStorage, persist } from 'zustand/middleware';
import { createWithEqualityFn } from 'zustand/traditional';

import { DeviceConfigsCardNetworkInfo } from '../../../shared/components/network/DeviceConfigsCard/types';
import { AddDeviceResponseDevice } from '../../../shared/types';
import { AddDeviceNavigationEvent, AddDeviceStep } from '../types';

const defaultValues: StoreValues = {
  navigationSubject: new Subject(),
  currentStep: AddDeviceStep.CHOOSE_METHOD,
  userData: undefined,
  loading: false,
  publicKey: undefined,
  privateKey: undefined,
  device: undefined,
  networks: undefined,
  clientSetup: undefined,
};

export const useAddDevicePageStore = createWithEqualityFn<Store>()(
  persist(
    (set) => ({
      ...defaultValues,
      reset: () => set(defaultValues),
      init: (userData) => {
        set({ ...defaultValues, userData });
      },
      setState: (values) => set({ ...values }),
      setStep: (step, values) => {
        set({ ...values, currentStep: step });
      },
    }),
    {
      name: 'add-device-store',
      partialize: (store) => omit(store, ['navigationSubject', 'loading']),
      storage: createJSONStorage(() => sessionStorage),
    },
  ),
  Object.is,
);

type Store = StoreValues & StoreMethods;

type StoreValues = {
  navigationSubject: Subject<AddDeviceNavigationEvent>;
  currentStep: AddDeviceStep;
  loading: boolean;
  privateKey?: string;
  publicKey?: string;
  device?: AddDeviceResponseDevice;
  networks?: DeviceConfigsCardNetworkInfo[];
  userData?: {
    id: number;
    username: string;
    reservedDevices: string[];
    email: string;
    // this should be current path that user entered add-device page from, due to brave blocking history relative back doesn't work correctly.
    originRoutePath: string;
  };
  clientSetup?: {
    token: string;
    url: string;
  };
};

type StoreMethods = {
  init: (userData: StoreValues['userData']) => void;
  reset: () => void;
  setState: (values: Partial<StoreValues>) => void;
  setStep: (step: AddDeviceStep, values?: Partial<StoreValues>) => void;
};
