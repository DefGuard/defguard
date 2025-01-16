import { createWithEqualityFn } from 'zustand/traditional';

import { StandaloneDevice } from '../../../../shared/types';

const defaults: StoreValues = {
  visible: false,
};

export const useStandaloneDeviceConfigModal = createWithEqualityFn<Store>(
  (set) => ({
    ...defaults,
    open: (data) => set({ data, visible: true }),
    close: () => set({ visible: false }),
    reset: () => set(defaults),
  }),
  Object.is,
);

type Store = StoreValues & StoreMethods;
type StoreValues = {
  visible: boolean;
  data?: {
    device: StandaloneDevice;
    config: string;
  };
};

type StoreMethods = {
  open: (values: StoreValues['data']) => void;
  close: () => void;
  reset: () => void;
};
