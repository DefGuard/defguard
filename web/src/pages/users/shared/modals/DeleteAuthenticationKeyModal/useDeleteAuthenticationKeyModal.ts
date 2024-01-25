import { createWithEqualityFn } from 'zustand/traditional';

import { AuthenticationKey } from '../../../../../shared/types';

const defaultValues: StoreValues = {
  visible: false,
  authenticationKey: undefined,
};

export const useDeleteAuthenticationKeyModal = createWithEqualityFn<Store>(
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
  authenticationKey?: AuthenticationKey;
};

type StoreMethods = {
  setState: (values: Partial<StoreValues>) => void;
  open: (values: Partial<StoreValues>) => void;
  close: () => void;
};

type Store = StoreValues & StoreMethods;
