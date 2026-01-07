import { createWithEqualityFn } from 'zustand/traditional';

const defaultValues: StoreValues = {
  visible: false,
  provisionerId: undefined,
};

export const useDeleteProvisionerModal = createWithEqualityFn<Store>(
  (set) => ({
    ...defaultValues,
    open: (values) => set({ ...defaultValues, ...values, visible: true }),
    close: () => set({ visible: false }),
    reset: () => set(defaultValues),
  }),
  Object.is,
);

type Store = StoreValues & StoreMethods;

type StoreValues = {
  visible: boolean;
  provisionerId?: string;
};

type StoreMethods = {
  open: (values?: Partial<StoreValues>) => void;
  close: () => void;
  reset: () => void;
};
