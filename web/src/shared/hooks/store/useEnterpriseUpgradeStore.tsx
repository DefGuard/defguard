import { isObject, pick } from 'lodash-es';
import { persist } from 'zustand/middleware';
import { createWithEqualityFn } from 'zustand/traditional';

import { EnterpriseUpgradeToast } from '../../components/Layout/EnterpriseUpgradeToast/EnterpriseUpgradeToast';
import { EnterpriseUpgradeToastMeta } from '../../components/Layout/EnterpriseUpgradeToast/types';
import { versionUpdateToastMetaSchema } from '../../components/Layout/VersionUpdateToast/types';
import { ToastType } from '../../defguard-ui/components/Layout/ToastManager/Toast/types';
import { useToastsStore } from '../../defguard-ui/hooks/toasts/useToastStore';

const keysToPersist: Array<keyof StoreValues> = ['dismissal'];

const defaultState: StoreValues = {
  modalVisible: false,
  dismissal: undefined,
  // update: undefined,
};

const upgradeToastCustomId = 'enterprise-upgrade-toast';

export const useEnterpriseUpgradeStore = createWithEqualityFn<Store>()(
  persist(
    (set, get) => ({
      ...defaultState,
      setStore: (vals) => set(vals),
      show: () => {
        const state = get();
        if (!state.dismissal) {
          const { addToast, toasts } = useToastsStore.getState();
          // this is needed in order to not duplicate the version update toast upon page reload because toast is not dismissible and will otherwise appear again when update is checked for.
          const isIn = toasts.find((t) => {
            const meta = t.meta;
            if (meta) {
              const parseResult = versionUpdateToastMetaSchema.safeParse(meta);
              if (parseResult.success) {
                return parseResult.data.customId === upgradeToastCustomId;
              }
            }
            return false;
          });
          if (!isIn) {
            const meta: EnterpriseUpgradeToastMeta = {
              customId: upgradeToastCustomId,
            };
            addToast({
              customComponent: EnterpriseUpgradeToast,
              message: '',
              type: ToastType.INFO,
              meta,
            });
          }
        }
      },
    }),
    {
      name: 'enterprise-upgrade-store',
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

export type UpdateInfo = {
  version: string;
  critical: boolean;
  // Markdown
  notes: string;
  release_notes_url: string;
};

type StoreValues = {
  modalVisible: boolean;
  dismissal?: Dismissal;
  update?: UpdateInfo;
};

type StoreMethods = {
  setStore: (values: Partial<StoreValues>) => void;
  show: () => void;
};
