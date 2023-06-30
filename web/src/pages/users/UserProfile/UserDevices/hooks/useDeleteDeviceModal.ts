import { create } from 'zustand';

import { Device } from '../../../../../shared/types';

const defaultValues: StoreValues = {
  visible: false,
  device: undefined,
};

export const useDeleteDeviceModal = create<Store>((set) => ({
  ...defaultValues,
  setState: (values) => set((old) => ({ ...old, ...values })),
  open: (values) => set({ ...defaultValues, ...values }),
  close: () => set({ visible: false }),
}));

type StoreValues = {
  visible: boolean;
  device?: Device;
};

type StoreMethods = {
  setState: (values: Partial<StoreValues>) => void;
  open: (values: Partial<StoreValues>) => void;
  close: () => void;
};

type Store = StoreValues & StoreMethods;
