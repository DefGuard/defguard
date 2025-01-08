import { persist } from 'zustand/middleware';
import { createWithEqualityFn } from 'zustand/traditional';

import { UpgradeLicenseModalVariant } from './types';

const defaults: StoreValues = {
  visible: false,
  modalVariant: UpgradeLicenseModalVariant.ENTERPRISE_NOTICE,
  wasSeen: false,
};

export const useUpgradeLicenseModal = createWithEqualityFn<Store>()(
  persist(
    (set, get) => ({
      ...defaults,
      open: ({ modalVariant, force }) => {
        const { wasSeen } = get();
        if (!wasSeen || force) {
          set({ visible: true, modalVariant });
        }
      },
      close: () => set({ visible: false, wasSeen: true }),
      reset: () =>
        set({ visible: defaults.visible, modalVariant: defaults.modalVariant }),
    }),
    {
      name: 'upgrade-license-store',
      partialize: (s) => ({ wasSeen: s.wasSeen }),
      version: 1,
    },
  ),
  Object.is,
);

type Store = StoreValues & StoreMethods;
type StoreValues = {
  visible: boolean;
  modalVariant: UpgradeLicenseModalVariant;
  wasSeen: boolean;
};
type Open = Pick<StoreValues, 'modalVariant'> & {
  force?: boolean;
};

type StoreMethods = {
  open: (modalVariant: Open) => void;
  close: () => void;
  reset: () => void;
};
