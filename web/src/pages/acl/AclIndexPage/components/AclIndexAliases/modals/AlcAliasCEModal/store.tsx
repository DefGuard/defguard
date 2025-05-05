import { createWithEqualityFn } from 'zustand/traditional';

import { AclAlias } from '../../../../../types';

const defaults: StoreValues = {
  visible: false,
  alias: undefined,
};

export const useAclAliasCEModal = createWithEqualityFn<Store>(
  (set) => ({
    ...defaults,
    open: (vals) => {
      if (vals) {
        set({ ...defaults, ...vals, visible: true });
      } else {
        set({ ...defaults, visible: true });
      }
    },
    close: () => {
      set({ visible: false });
    },
    reset: () => {
      set(defaults);
    },
  }),
  Object.is,
);

type Store = StoreValues & StoreMethods;

type StoreValues = {
  visible: boolean;
  alias?: AclAlias;
};

type StoreMethods = {
  open: (vals?: Partial<StoreValues>) => void;
  close: () => void;
  reset: () => void;
};
