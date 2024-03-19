import { createWithEqualityFn } from 'zustand/traditional';

const defaultValues: StoreValues = {
  visible: false,
  keyData: undefined,
};

export const useDeleteAuthenticationKeyModal = createWithEqualityFn<Store>(
  (set) => ({
    ...defaultValues,
    open: (values) => set({ keyData: values, visible: true }),
    close: () => set({ visible: false }),
    reset: () => set(defaultValues),
  }),
  Object.is,
);

type StoreValues = {
  visible: boolean;
  keyData?: {
    id: number;
    type: 'ssh' | 'gpg' | 'yubikey';
    name: string;
    username: string;
  };
};

type StoreMethods = {
  open: (init: StoreValues['keyData']) => void;
  close: () => void;
  reset: () => void;
};

type Store = StoreValues & StoreMethods;
