import { createWithEqualityFn } from 'zustand/traditional';

import { Gateway } from '../../../../../shared/types';

const defaultValues: StoreValues = {
  visible: false,
  gateway: undefined,
};

export const useEditGatewayModal = createWithEqualityFn<Store>(
  (set) => ({
    ...defaultValues,
    setState: (values) => set((old) => ({ ...old, ...values })),
    open: (values) => set({ ...defaultValues, ...values }),
    close: () => set({ visible: false }),
  }),
  Object.is,
);

type StoreValues = {
  visible: boolean;
  gateway?: Gateway;
};

type StoreMethods = {
  setState: (values: Partial<StoreValues>) => void;
  open: (values: Partial<StoreValues>) => void;
  close: () => void;
};

type Store = StoreValues & StoreMethods;
