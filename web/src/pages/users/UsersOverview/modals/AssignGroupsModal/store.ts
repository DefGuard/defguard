import { Subject } from 'rxjs';
import { createWithEqualityFn } from 'zustand/traditional';

const defaults: StoreValues = {
  visible: false,
  usersToAssign: [],
  successSubject: new Subject(),
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
  // communicate back to overview that the assign succeeded and selection should be undone
  successSubject: Subject<void>;
};

type StoreMethods = {
  open: (users: number[]) => void;
  close: () => void;
  reset: () => void;
};
