import { pick } from 'lodash-es';
import { persist } from 'zustand/middleware';
import { createWithEqualityFn } from 'zustand/traditional';

const defaultState: StoreValues = {
  isOpen: false,
};

export const useNavigationStore = createWithEqualityFn<Store>()(
  persist(
    (set) => ({
      ...defaultState,
      setState: (values) => set((old) => ({ ...old, ...values })),
      reset: () => set(defaultState),
    }),
    {
      version: 1.5,
      name: 'navigation-store',
      partialize: (state) => pick(state, ['isOpen']),
    },
  ),
  Object.is,
);

type Store = StoreValues & StoreMethods;

type StoreValues = {
  isOpen: boolean;
};

type StoreMethods = {
  setState: (values: Partial<StoreValues>) => void;
  reset: () => void;
};
