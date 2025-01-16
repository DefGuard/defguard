import { createWithEqualityFn } from 'zustand/traditional';

import { StandaloneDevice } from '../../../shared/types';

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
  device?: StandaloneDevice;
};
type StoreMethods = {
  open: (device: StandaloneDevice) => void;
  close: () => void;
  reset: () => void;
};
