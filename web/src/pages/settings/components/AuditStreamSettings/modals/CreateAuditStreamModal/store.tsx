import { createWithEqualityFn } from 'zustand/traditional';

const defaults: StoreValues = {
  visible: false,
};

export const useCreateAuditStreamModalStore = createWithEqualityFn<Store>(
  (set) => ({
    ...defaults,
    open: () => set({ visible: true }),
    close: () => set({ visible: false }),
    reset: () => set(defaults),
  }),
  Object.is,
);

type Store = StoreValues & StoreMethods;

type StoreValues = {
  visible: boolean;
};

type StoreMethods = {
  open: () => void;
  close: () => void;
  reset: () => void;
};
