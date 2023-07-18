import { create } from 'zustand';

const defaultValues: StoreValues = {
  visible: false,
};

export const useChangeSelfPasswordModal = create<Store>((set) => ({
  ...defaultValues,
  open: () => set({ visible: true }),
  reset: () => set(defaultValues),
}));

type Store = StoreValues & StoreMethods;

type StoreValues = {
  visible: boolean;
};

type StoreMethods = {
  open: () => void;
  reset: () => void;
};
