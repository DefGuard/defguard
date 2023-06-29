import { create } from 'zustand';

import { Device } from '../../../../../shared/types';

const defaultValues: StoreValues = {
  visible: false,
  device: undefined,
};

export const useDeleteDeviceModal = create<Store>((set) => ({
  ...defaultValues,
  setState: (values) => set((old) => ({ ...old, ...values })),
  reset: () => set(defaultValues),
}));

type StoreValues = {
  visible: boolean;
  device?: Device;
};

type StoreMethods = {
  setState: (values: Partial<StoreValues>) => void;
  reset: () => void;
};

type Store = StoreValues & StoreMethods;
