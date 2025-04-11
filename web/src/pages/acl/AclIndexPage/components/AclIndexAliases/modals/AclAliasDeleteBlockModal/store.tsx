import { createWithEqualityFn } from 'zustand/traditional';

import { AclAlias } from '../../../../../types';

const defaults: StoreValues = {
  visible: false,
  alias: undefined,
  rulesNames: [],
};

export const useAclAliasDeleteBlockModal = createWithEqualityFn<Store>(
  (set) => ({
    ...defaults,
    open: (alias, rules) => set({ alias: alias, rulesNames: rules, visible: true }),
    close: () => set({ visible: false }),
    reset: () => set(defaults),
  }),
  Object.is,
);

type Store = StoreValues & StoreMethods;

type StoreValues = {
  visible: boolean;
  rulesNames: string[];
  alias?: AclAlias;
};

type StoreMethods = {
  open: (alias: AclAlias, rules: string[]) => void;
  close: () => void;
  reset: () => void;
};
