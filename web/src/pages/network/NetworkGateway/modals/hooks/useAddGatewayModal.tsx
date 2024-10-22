import { createWithEqualityFn } from 'zustand/traditional';

const defaultValues: StoreValues = {
  visible: false,
  url: '',
};

export const useAddGatewayModal = createWithEqualityFn<Store>(
  (set, get) => ({
    ...defaultValues,
    open: () => set({ ...defaultValues, visible: true }),
    close: () => set({ visible: false }),
    setState: (values) => set((old) => ({ ...old, ...values })),
    reset: () => set(defaultValues),
  }),
  Object.is,
);

type Store = StoreValues & StoreMethods;

type StoreValues = {
  visible: boolean;
  url: string;
};

type StoreMethods = {
  open: () => void;
  close: () => void;
  reset: () => void;
  setState: (values: Partial<StoreValues>) => void;
};
