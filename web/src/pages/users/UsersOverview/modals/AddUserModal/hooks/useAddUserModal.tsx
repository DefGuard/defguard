import { createWithEqualityFn } from 'zustand/traditional';

import { StartEnrollmentResponse, User } from '../../../../../../shared/types';

const defaultValues: StoreValues = {
  visible: false,
  step: 0,
  user: undefined,
};

export const useAddUserModal = createWithEqualityFn<Store>(
  (set, get) => ({
    ...defaultValues,
    open: () => set({ ...defaultValues, visible: true }),
    close: () => set({ visible: false }),
    nextStep: () => set({ step: get().step + 1 }),
    setState: (values) => set((old) => ({ ...old, ...values })),
    reset: () => set(defaultValues),
  }),
  Object.is,
);

type Store = StoreValues & StoreMethods;

type StoreValues = {
  visible: boolean;
  step: number;
  user?: User;
  tokenResponse?: StartEnrollmentResponse;
};

type StoreMethods = {
  open: () => void;
  close: () => void;
  reset: () => void;
  nextStep: () => void;
  setState: (values: Partial<StoreValues>) => void;
};
