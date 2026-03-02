import { create } from 'zustand';
import type { ApplicationInfo, SessionInfo, SettingsEssentials } from '../api/types';

type StoreValues = {
  navigationOpen: boolean;
  appInfo: ApplicationInfo;
  settingsEssentials?: SettingsEssentials;
  sessionInfo: SessionInfo;
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
  sessionInfo: {
    authorized: false,
    isAdmin: false,
    wizard_flags: null,
  },
  settingsEssentials: undefined,
};

export const useApp = create<Store>(() => ({ ...defaults }));
