import { createWithEqualityFn } from 'zustand/traditional';

import { AclRuleInfo } from '../../shared/types';

const defaults: StoreValues = {
  formRuleInitialValue: {
    aliases: [],
    all_networks: false,
    allow_all_users: false,
    allowed_devices: [],
    allowed_groups: [],
    allowed_users: [],
    denied_devices: [],
    denied_groups: [],
    denied_users: [],
    deny_all_users: false,
    destination: '',
    id: 0,
    name: '',
    networks: [],
    ports: '',
    protocols: [],
    expires: undefined,
  },
};

export const useAclContext = createWithEqualityFn<Store>(
  (set) => ({
    ...defaults,
    setValues: (values) => {
      set(values);
    },
    reset: () => set(defaults),
  }),
  Object.is,
);

type Store = StoreValues & StoreMethods;

type StoreValues = {
  formRuleInitialValue: AclRuleInfo;
};

type StoreMethods = {
  setValues: (values: Partial<StoreValues>) => void;
  reset: () => void;
};
