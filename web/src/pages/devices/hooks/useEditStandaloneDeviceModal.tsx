import { createWithEqualityFn } from 'zustand/traditional';

import { StandaloneDevice } from '../../../shared/types';

const defaults: StoreValues = {
  visible: false,
  device: undefined,
};

export const useEditStandaloneDeviceModal = createWithEqualityFn<Store>(
  (set) => ({
    ...defaults,
    open: (device) => set({ device: device, visible: true }),
    close: () => set({ visible: false }),
    reset: () => set(defaults),
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
