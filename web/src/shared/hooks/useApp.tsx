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
    license_info: {
      any_limit_exceeded: false,
      enterprise: true,
      is_enterprise_free: true,
      limits_exceeded: {
        device: false,
        user: false,
        wireguard_network: false,
      },
    },
    network_present: false,
    smtp_enabled: false,
    version: '',
  },
  settingsEssentials: undefined,
};

export const useApp = create<Store>(() => ({ ...defaults }));
