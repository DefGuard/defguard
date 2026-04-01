import { create } from 'zustand';
import type { ApplicationInfo, SettingsEssentials, WizardState } from '../api/types';

type StoreValues = {
  navigationOpen: boolean;
  tutorialsModalOpen: boolean;
  appInfo: ApplicationInfo;
  settingsEssentials?: SettingsEssentials;
  wizardState?: WizardState;
};

type Store = StoreValues;

const defaults: StoreValues = {
  navigationOpen: true,
  tutorialsModalOpen: false,
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
  wizardState: undefined,
};

export const useApp = create<Store>(() => ({ ...defaults }));
