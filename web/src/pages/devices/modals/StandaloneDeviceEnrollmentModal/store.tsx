import { createWithEqualityFn } from 'zustand/traditional';

import { StandaloneDevice, StartEnrollmentResponse } from '../../../../shared/types';

const defaults: StoreValues = {
  visible: false,
  data: undefined,
};

export const useStandaloneDeviceEnrollmentModal = createWithEqualityFn<Store>(
  (set) => ({
    ...defaults,
    open: (data) => set({ data: data, visible: true }),
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
    enrollment: StartEnrollmentResponse;
  };
};
type StoreMethods = {
  open: (data: StoreValues['data']) => void;
  close: () => void;
  reset: () => void;
};
