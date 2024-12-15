import { createWithEqualityFn } from 'zustand/traditional';

import { MockDevice } from './useDevicesPage';

const defaultValues: StoreValues = {
  visible: false,
  device: undefined,
};

export const useDeleteStandaloneDeviceModal = createWithEqualityFn<Store>(
  (set) => ({
    ...defaultValues,
    open: (device) => set({ visible: true, device }),
    close: () => set({ visible: false }),
    reset: () => set(defaultValues),
  }),
  Object.is,
);

type Store = StoreValues & StoreMethods;

type StoreValues = {
  visible: boolean;
  device?: MockDevice;
};
type StoreMethods = {
  open: (device: MockDevice) => void;
  close: () => void;
  reset: () => void;
};
