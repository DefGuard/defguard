import { createWithEqualityFn } from 'zustand/traditional';

const defaultValues: StoreValues = {
  visible: false,
};

export const useEmailMFAModal = createWithEqualityFn<Store>(
  (set) => ({
    ...defaultValues,
    open: (data) => set({ ...data, visible: true }),
    close: () => set({ visible: false }),
    reset: () => set(defaultValues),
  }),
  Object.is,
);

type Store = StoreValues & StoreMethods;

type StoreValues = {
  visible: boolean;
};

type StoreMethods = {
  open: (values?: Partial<StoreValues>) => void;
  close: () => void;
  reset: () => void;
};
