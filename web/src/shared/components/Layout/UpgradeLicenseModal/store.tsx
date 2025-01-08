import { createWithEqualityFn } from 'zustand/traditional';

import { UpgradeLicenseModalVariant } from './types';

const defaults: StoreValues = {
  visible: false,
  modalVariant: UpgradeLicenseModalVariant.ENTERPRISE_NOTICE,
};

export const useUpgradeLicenseModal = createWithEqualityFn<Store>(
  (set) => ({
    ...defaults,
    open: ({ modalVariant }) => {
      set({ visible: true, modalVariant });
    },
    close: () => set({ visible: false }),
    reset: () => set({ visible: defaults.visible, modalVariant: defaults.modalVariant }),
  }),
  Object.is,
);

type Store = StoreValues & StoreMethods;
type StoreValues = {
  visible: boolean;
  modalVariant: UpgradeLicenseModalVariant;
};
type Open = Pick<StoreValues, 'modalVariant'>;

type StoreMethods = {
  open: (modalVariant: Open) => void;
  close: () => void;
  reset: () => void;
};
