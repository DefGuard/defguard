import { createWithEqualityFn } from 'zustand/traditional';

import { User } from '../../../../../../shared/types';

const defaultValues: StoreValues = {
  isOpen: false,
  user: undefined,
};

export const useEnrollmentModalStore = createWithEqualityFn<Store>(
  (set) => ({
    ...defaultValues,
    open: (user) => set({ isOpen: true, user }),
    close: () => set({ isOpen: false }),
    reset: () => set(defaultValues),
  }),
  Object.is,
);

type Store = StoreValues & StoreMethods;

type StoreValues = {
  isOpen: boolean;
  user?: User;
};

type StoreMethods = {
  open: (user: User) => void;
  close: () => void;
  reset: () => void;
};
