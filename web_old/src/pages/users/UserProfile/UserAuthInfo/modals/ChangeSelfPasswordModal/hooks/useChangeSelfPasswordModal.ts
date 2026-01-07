import { createWithEqualityFn } from 'zustand/traditional';

const defaultValues: StoreValues = {
  visible: false,
};

export const useChangeSelfPasswordModal = createWithEqualityFn<Store>(
  (set) => ({
    ...defaultValues,
    open: () => set({ visible: true }),
    reset: () => set(defaultValues),
  }),
  Object.is,
);

type Store = StoreValues & StoreMethods;

type StoreValues = {
  visible: boolean;
};

type StoreMethods = {
  open: () => void;
  reset: () => void;
};
