import { createWithEqualityFn } from 'zustand/traditional';

import { GroupInfo } from '../../../../../shared/types';

const defaults: StoreValues = {
  visible: false,
  groupInfo: undefined,
};

export const useAddGroupModal = createWithEqualityFn<Store>(
  (set) => ({
    ...defaults,
    open: (group) => set({ visible: true, groupInfo: group }),
    close: () => set({ visible: false }),
  }),
  Object.is,
);

type Store = StoreValues & StoreMethods;

type StoreValues = {
  visible: boolean;
  groupInfo?: GroupInfo;
};

type StoreMethods = {
  open: (group?: GroupInfo) => void;
  close: () => void;
};
