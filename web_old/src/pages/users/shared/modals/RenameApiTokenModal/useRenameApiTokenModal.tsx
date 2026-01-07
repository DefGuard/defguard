import { createWithEqualityFn } from 'zustand/traditional';

const defaults: StoreValues = {
  visible: false,
  tokenData: undefined,
};

export const useRenameApiTokenModal = createWithEqualityFn<Store>(
  (set) => ({
    ...defaults,
    open: (data) => set({ visible: true, tokenData: data }),
    close: () => set({ visible: false }),
    reset: () => set(defaults),
  }),
  Object.is,
);

type Store = StoreValues & StoreMethods;

type TokenData = {
  id: number;
  username: string;
  name: string;
};

type StoreValues = {
  visible: boolean;
  tokenData?: TokenData;
};

type StoreMethods = {
  open: (data: TokenData) => void;
  close: () => void;
  reset: () => void;
};
