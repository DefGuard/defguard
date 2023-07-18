import { pick } from 'lodash-es';
import { create } from 'zustand';
import { persist } from 'zustand/middleware';

const defaultState: StoreValues = {
  isOpen: false,
};

export const useNavigationStore = create<Store>()(
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
);

type Store = StoreValues & StoreMethods;

type StoreValues = {
  isOpen: boolean;
};

type StoreMethods = {
  setState: (values: Partial<StoreValues>) => void;
  reset: () => void;
};
