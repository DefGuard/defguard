import { createWithEqualityFn } from 'zustand/traditional';

const defaults: StoreValues = {
  visible: false,
  keyData: undefined,
};

export const useRenameAuthenticationKeyModal = createWithEqualityFn<Store>(
  (set) => ({
    ...defaults,
    open: (data) => set({ visible: true, keyData: data }),
    close: () => set({ visible: false }),
    reset: () => set(defaults),
  }),
  Object.is,
);

type SupportedKeys = 'ssh' | 'gpg' | 'yubikey';

type Store = StoreValues & StoreMethods;

type KeyData = {
  id: number;
  username: string;
  name: string;
  key_type: SupportedKeys;
};

type StoreValues = {
  visible: boolean;
  keyData?: KeyData;
};

type StoreMethods = {
  open: (data: KeyData) => void;
  close: () => void;
  reset: () => void;
};
