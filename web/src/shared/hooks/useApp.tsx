import { create } from 'zustand';
import type { ApplicationInfo, SettingsEssentials } from '../api/types';

type StoreValues = {
  navigationOpen: boolean;
  appInfo: ApplicationInfo;
  settingsEssentials?: SettingsEssentials;
};

type Store = StoreValues;

const defaults: StoreValues = {
  navigationOpen: true,
  appInfo: {
    external_openid_enabled: false,
    ldap_info: {
      ad: false,
      enabled: false,
    },
    network_present: false,
    smtp_enabled: false,
    version: '',
  },
  settingsEssentials: undefined,
};

export const useApp = create<Store>(() => ({ ...defaults }));
