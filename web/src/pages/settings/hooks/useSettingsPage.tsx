import { createWithEqualityFn } from 'zustand/traditional';

import { Settings } from '../../../shared/types';

const defaultValues: StoreValues = {
  settings: undefined,
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
};

type StoreMethods = {
  setState: (data: Partial<StoreValues>) => void;
  reset: () => void;
};
