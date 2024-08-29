import { pick } from 'lodash-es';
import { createJSONStorage, persist } from 'zustand/middleware';
import { createWithEqualityFn } from 'zustand/traditional';

import { Locales } from '../../../i18n/i18n-types';
import { AppInfo, EnterpriseStatus, SettingsEnterprise, SettingsEssentials } from '../../types';

const defaultValues: StoreValues = {
  settings: undefined,
  language: undefined,
  appInfo: undefined,
  enterprise_status: undefined,
  enterprise_settings: undefined,
};

const persistKeys: Array<keyof StoreValues> = ['language'];

export const useAppStore = createWithEqualityFn<Store>()(
  persist(
    (set) => ({
      ...defaultValues,
      setState: (data) => set(data),
    }),
    {
      name: 'app-store',
      version: 0.2,
      partialize: (store) => pick(store, persistKeys),
      storage: createJSONStorage(() => sessionStorage),
    },
  ),
  Object.is,
);

type Store = StoreValues & StoreMethods;

type StoreValues = {
  settings?: SettingsEssentials;
  language?: Locales;
  appInfo?: AppInfo;
  enterprise_status?: EnterpriseStatus;
  enterprise_settings?: SettingsEnterprise;
};

type StoreMethods = {
  setState: (values: Partial<StoreValues>) => void;
};
