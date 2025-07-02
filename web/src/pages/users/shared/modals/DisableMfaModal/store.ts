import { createWithEqualityFn } from 'zustand/traditional';

import { User } from '../../../../../shared/types';

const defaults: StoreValues = {
  visible: false,
  user: undefined,
};

export const useDisableMfaModal = createWithEqualityFn<Store>((set) => ({
  ...defaults,
  open: (user: User) => set({ visible: true, user }),
  setIsOpen: (v: boolean) => set({ visible: v }),
  close: () => set({ visible: false }),
  reset: () => set(defaults),
}));

type Store = StoreValues & StoreMethods;

type StoreValues = {
  visible: boolean;
  user?: User;
};

type StoreMethods = {
  open: (user: User) => void;
  setIsOpen: (v: boolean) => void;
  close: () => void;
  reset: () => void;
};
