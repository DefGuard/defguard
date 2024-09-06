import { createWithEqualityFn } from 'zustand/traditional';

const defaultValues: StoreValues = {
  emailChange: {
    open: false,
    accepted: false,
  },
  usernameChange: {
    open: false,
    accepted: false,
  },
};

export const useProfileDetailsWarningModal = createWithEqualityFn<Store>(
  (set) => ({
    ...defaultValues,
    open: (modal) => set({ [modal]: { open: true, accepted: false } }),
    close: (modal) => set({ [modal]: { open: false, accepted: false } }),
    accept: (modal) => set({ [modal]: { open: false, accepted: true } }),
    reset: () => set(defaultValues),
  }),
  Object.is,
);

type Store = StoreValues & StoreMethods;

type StoreValues = {
  emailChange: {
    open: boolean;
    accepted: boolean;
  };
  usernameChange: {
    open: boolean;
    accepted: boolean;
  };
};

type StoreMethods = {
  open: (modal: 'usernameChange' | 'emailChange') => void;
  close: (modal: 'usernameChange' | 'emailChange') => void;
  accept: (modal: 'usernameChange' | 'emailChange') => void;
  reset: () => void;
};
