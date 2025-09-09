import { createWithEqualityFn } from 'zustand/traditional';
import type { OutdatedComponents } from '../../../types';

const defaultValues: StoreValues = {
  componentsInfo: { gateways: []},
  visible: false,
};

export const useOutdatedComponentsModal = createWithEqualityFn<Store>((set) => ({
  ...defaultValues,
  close: () => set({ visible: false }),
  open: (data) => set({ visible: true, componentsInfo: data }),
  reset: () => set(defaultValues),
}));

type Store = StoreMethods & StoreValues;

type StoreMethods = {
  open: (initData: OutdatedComponents) => void;
  close: () => void;
  reset: () => void;
};

type StoreValues = {
  visible: boolean;
  componentsInfo: OutdatedComponents;
};
