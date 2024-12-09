import { isObject, pick } from 'lodash-es';
import { persist } from 'zustand/middleware';
import { createWithEqualityFn } from 'zustand/traditional';

import { VersionUpdateToast } from '../../components/Layout/VersionUpdateToast/VersionUpdateToast';
import { ToastType } from '../../defguard-ui/components/Layout/ToastManager/Toast/types';
import { useToastsStore } from '../../defguard-ui/hooks/toasts/useToastStore';

const keysToPersist: Array<keyof StoreValues> = ['dismissal'];

const defaultState: StoreValues = {
  modalVisible: false,
  dismissal: undefined,
  update: undefined,
};

export const useUpdatesStore = createWithEqualityFn<Store>()(
  persist(
    (set, get) => ({
      ...defaultState,
      setStore: (vals) => set(vals),
      openModal: () => set({ modalVisible: true }),
      closeModal: () => set({ modalVisible: false }),
      setUpdate: (update) => {
        const state = get();
        if (!state.dismissal || state.dismissal.version !== update.version) {
          useToastsStore.getState().addToast({
            customComponent: VersionUpdateToast,
            message: '',
            type: ToastType.INFO,
          });
          set({ update: update });
        } else {
          set({ update: update });
        }
      },
    }),
    {
      name: 'updates-store',
      version: 1,
      partialize: (s) => pick(s, keysToPersist),
    },
  ),
  isObject,
);

type Store = StoreValues & StoreMethods;

type Dismissal = {
  version: string;
  dismissedAt: string;
};

type UpdateInfo = {
  version: string;
  critical: boolean;
  // Markdown
  notes: string;
  releaseLink: string;
};

type StoreValues = {
  modalVisible: boolean;
  dismissal?: Dismissal;
  update?: UpdateInfo;
};

type StoreMethods = {
  setStore: (values: Partial<StoreValues>) => void;
  openModal: () => void;
  closeModal: () => void;
  setUpdate: (value: NonNullable<StoreValues['update']>) => void;
};
