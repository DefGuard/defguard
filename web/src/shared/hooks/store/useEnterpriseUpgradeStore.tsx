import { isObject } from 'lodash-es';
import { persist } from 'zustand/middleware';
import { createWithEqualityFn } from 'zustand/traditional';

import { EnterpriseUpgradeToast } from '../../components/Layout/EnterpriseUpgradeToast/EnterpriseUpgradeToast';
import { EnterpriseUpgradeToastMeta } from '../../components/Layout/EnterpriseUpgradeToast/types';
import { versionUpdateToastMetaSchema } from '../../components/Layout/VersionUpdateToast/types';
import { ToastType } from '../../defguard-ui/components/Layout/ToastManager/Toast/types';
import { useToastsStore } from '../../defguard-ui/hooks/toasts/useToastStore';

const upgradeToastCustomId = 'enterprise-upgrade-toast';

export const useEnterpriseUpgradeStore = createWithEqualityFn<Store>()(
  persist(
    () => ({
      show: () => {
        const { addToast, toasts } = useToastsStore.getState();
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
      },
    }),
    {
      name: 'enterprise-upgrade-store',
      version: 1,
    },
  ),
  isObject,
);

type Store = {
  show: () => void;
};
