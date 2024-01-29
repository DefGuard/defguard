import { createWithEqualityFn } from 'zustand/traditional';

const defaults: StoreValues = {
  visible: false,
  usersToAssign: [],
};

export const useAssignGroupsModal = createWithEqualityFn<Store>((set) => ({
  ...defaults,
  open: (users: number[]) => set({ visible: true, usersToAssign: users }),
  close: () => set({ visible: false }),
  reset: () => set(defaults),
}));

type Store = StoreValues & StoreMethods;

type StoreValues = {
  visible: boolean;
  usersToAssign: number[];
};

type StoreMethods = {
  open: (users: number[]) => void;
  close: () => void;
  reset: () => void;
};
