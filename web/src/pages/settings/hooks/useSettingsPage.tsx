import { createWithEqualityFn } from 'zustand/traditional';

import { EnterpriseInfo, Settings } from '../../../shared/types';

const defaultValues: StoreValues = {
  settings: undefined,
  enterpriseInfo: undefined,
  freeLicense: true,
};

export const useSettingsPage = createWithEqualityFn<Store>(
  (set) => ({
    ...defaultValues,
    setState: (data) => set(data),
    reset: () => set(defaultValues),
  }),
  Object.is,
);

type Store = StoreValues & StoreMethods;

type StoreValues = {
  settings?: Settings;
  enterpriseInfo?: EnterpriseInfo;
  freeLicense: boolean;
};

type StoreMethods = {
  setState: (data: Partial<StoreValues>) => void;
  reset: () => void;
};
