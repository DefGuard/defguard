import { createWithEqualityFn } from 'zustand/traditional';

import { AclAlias } from '../../../../../types';

const defaults: StoreValues = {
  visible: false,
  alias: undefined,
};

export const useAclAliasDeleteBlockModal = createWithEqualityFn<Store>(
  (set) => ({
    ...defaults,
    open: (alias) => set({ alias: alias, visible: true }),
    close: () => set({ visible: false }),
    reset: () => set(defaults),
  }),
  Object.is,
);

type Store = StoreValues & StoreMethods;

type StoreValues = {
  visible: boolean;
  alias?: AclAlias;
};

type StoreMethods = {
  open: (alias: AclAlias) => void;
  close: () => void;
  reset: () => void;
};
