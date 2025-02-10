import { createWithEqualityFn } from 'zustand/traditional';

import { User } from '../../../../../shared/types';

const defaults: StoreValues = {
  visible: false,
  provisioningInProgress: false,
  user: undefined,
};

export const useAddApiTokenModal = createWithEqualityFn<Store>(
  (set) => ({
    ...defaults,
    open: (initVals) => set({ ...defaults, ...initVals, visible: true }),
    close: () => set({ visible: false }),
    reset: () => set(defaults),
    setState: (values) => set(values),
  }),
  Object.is,
);

type Store = StoreValues & StoreMethods;

type StoreValues = {
  visible: boolean;
  provisioningInProgress: boolean;
  user?: User;
};

type StoreMethods = {
  open: (state: Partial<StoreValues>) => void;
  close: () => void;
  reset: () => void;
  setState: (newValues: Partial<StoreValues>) => void;
};
