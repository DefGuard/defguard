import { createWithEqualityFn } from 'zustand/traditional';

import { Settings } from '../../../shared/types';

const defaultValues: StoreValues = {
  settings: undefined,
};

export const useEnrollmentStore = createWithEqualityFn<Store>(
  (set, get) => ({
    ...defaultValues,
    setState: (newValues) => set({ ...get(), ...newValues }),
  }),
  Object.is,
);

type Store = StoreValues & StoreMethods;

type StoreValues = {
  settings?: Settings;
};

type StoreMethods = {
  setState: (newValues: Partial<StoreValues>) => void;
};
