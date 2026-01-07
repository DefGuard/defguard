import { createWithEqualityFn } from 'zustand/traditional';

const defaults: StoreValues = {
  groups: [],
  visible: false,
};

export const useUserGroupsListModal = createWithEqualityFn<Store>(
  (set) => ({
    ...defaults,
    open: (g) => set({ visible: true, groups: g }),
    close: () => set({ visible: false }),
    reset: () => set(defaults),
  }),
  Object.is,
);

type Store = StoreValues & StoreMethods;

type StoreValues = {
  visible: boolean;
  groups: string[];
};

type StoreMethods = {
  open: (groups: string[]) => void;
  close: () => void;
  reset: () => void;
};
