import { create } from 'zustand';

interface StoreValues {
  isOpen: boolean;
}

const defaults: StoreValues = {
  isOpen: false,
};

interface Store extends StoreValues {
  open: () => void;
  reset: () => void;
}

export const useAddNewDeviceModal = create<Store>((set) => ({
  ...defaults,
  reset: () => set(defaults),
  open: () => set({ isOpen: true }),
}));
