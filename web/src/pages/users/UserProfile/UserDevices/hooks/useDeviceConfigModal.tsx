import { createWithEqualityFn } from 'zustand/traditional';

import { DeviceConfigsCardNetworkInfo } from '../../../../../shared/components/network/DeviceConfigsCard/types';

const defaultValues: StoreValues = {
  isOpen: false,
  userId: undefined,
  publicKey: undefined,
  deviceId: undefined,
  networks: undefined,
  deviceName: undefined,
};

export const useDeviceConfigModal = createWithEqualityFn<Store>(
  (set) => ({
    ...defaultValues,
    open: (values) => set({ ...values, isOpen: true }),
    close: () => set({ isOpen: false }),
    reset: () => set(defaultValues),
  }),
  Object.is,
);

type Store = StoreValues & StoreMethods;

type StoreValues = {
  isOpen: boolean;
  publicKey?: string;
  userId?: number;
  deviceId?: number;
  networks?: DeviceConfigsCardNetworkInfo[];
  deviceName?: string;
};

type StoreMethods = {
  open: (values: Partial<StoreValues>) => void;
  close: () => void;
  reset: () => void;
};
