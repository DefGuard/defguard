import dayjs from 'dayjs';
import { persist } from 'zustand/middleware';
import { createWithEqualityFn } from 'zustand/traditional';

import { UpgradeLicenseModalVariant } from './types';

//minutes
const modalTimeout = 30;

const defaults: StoreValues = {
  visible: false,
  modalVariant: UpgradeLicenseModalVariant.ENTERPRISE_NOTICE,
  lastClosed: undefined,
};

export const useUpgradeLicenseModal = createWithEqualityFn<Store>()(
  persist(
    (set, get) => ({
      ...defaults,
      open: ({ modalVariant }) => {
        const { lastClosed } = get();
        if (
          lastClosed !== undefined &&
          modalVariant === UpgradeLicenseModalVariant.LICENSE_LIMIT
        ) {
          const past = dayjs(lastClosed).utc();
          const now = dayjs().utc();
          const diff = now.diff(past, 'minutes');
          if (diff >= modalTimeout) {
            set({ visible: true, modalVariant });
          }
        } else {
          set({ visible: true, modalVariant });
        }
      },
      close: () => {
        const { modalVariant } = get();
        if (modalVariant === UpgradeLicenseModalVariant.LICENSE_LIMIT) {
          set({ visible: false, lastClosed: dayjs().utc().toISOString() });
        } else {
          set({ visible: false });
        }
      },
      reset: () =>
        set({ visible: defaults.visible, modalVariant: defaults.modalVariant }),
    }),
    {
      name: 'upgrade-license-modal',
      version: 1,
      partialize: (s) => ({ lastOpened: s.lastClosed }),
    },
  ),
  Object.is,
);

type Store = StoreValues & StoreMethods;
type StoreValues = {
  visible: boolean;
  modalVariant: UpgradeLicenseModalVariant;
  lastClosed?: string;
};
type Open = Pick<StoreValues, 'modalVariant'>;

type StoreMethods = {
  open: (modalVariant: Open) => void;
  close: () => void;
  reset: () => void;
};
