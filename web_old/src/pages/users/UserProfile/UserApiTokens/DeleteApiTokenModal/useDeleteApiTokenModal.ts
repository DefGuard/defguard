import { createWithEqualityFn } from 'zustand/traditional';

const defaultValues: StoreValues = {
  visible: false,
  tokenData: undefined,
};

export const useDeleteApiTokenModal = createWithEqualityFn<Store>(
  (set) => ({
    ...defaultValues,
    open: (values) => set({ tokenData: values, visible: true }),
    close: () => set({ visible: false }),
    reset: () => set(defaultValues),
  }),
  Object.is,
);

type StoreValues = {
  visible: boolean;
  tokenData?: {
    id: number;
    name: string;
    username: string;
  };
};

type StoreMethods = {
  open: (init: StoreValues['tokenData']) => void;
  close: () => void;
  reset: () => void;
};

type Store = StoreValues & StoreMethods;
