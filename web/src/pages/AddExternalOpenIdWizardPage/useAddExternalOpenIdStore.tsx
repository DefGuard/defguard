import { omit } from 'lodash-es';
import { create } from 'zustand';
import { createJSONStorage, persist } from 'zustand/middleware';
import {
  type AddOpenIdProvider,
  DirectorySyncBehavior,
  DirectorySyncTarget,
  OpenIdProviderUsernameHandling,
} from '../../shared/api/types';
import {
  externalProviderName,
  googleProviderBaseUrl,
  jumpcloudProviderBaseUrl,
  SUPPORTED_SYNC_PROVIDERS,
} from '../../shared/constants';
import { ExternalProvider, type ExternalProviderValue } from '../settings/shared/types';
import { AddExternalProviderStep, type AddExternalProviderStepValue } from './types';

type ProviderState = AddOpenIdProvider & {
  microsoftTenantId?: string | null;
};

interface StoreValues {
  provider: ExternalProviderValue;
  activeStep: AddExternalProviderStepValue;
  providerState: ProviderState;
  testResult: boolean | null;
  testMessage: string | null;
}

export const addExternalOpenIdStoreDefaults: StoreValues = {
  provider: ExternalProvider.Custom,
  activeStep: 'client-settings',
  testResult: null,
  testMessage: null,
  providerState: {
    name: '',
    display_name: '',
    admin_email: '',
    base_url: '',
    client_id: '',
    client_secret: '',
    create_account: false,
    microsoftTenantId: null,
    directory_sync_group_match: null,
    google_service_account_email: null,
    google_service_account_key: null,
    okta_dirsync_client_id: null,
    okta_private_jwk: null,
    jumpcloud_api_key: null,
    directory_sync_enabled: false,
    directory_sync_interval: 600,
    directory_sync_target: DirectorySyncTarget.All,
    directory_sync_admin_behavior: DirectorySyncBehavior.Keep,
    directory_sync_user_behavior: DirectorySyncBehavior.Keep,
    prefetch_users: false,
    username_handling: OpenIdProviderUsernameHandling.RemoveForbidden,
  },
};

interface Store extends StoreValues {
  reset: () => void;
  initialize: (provider: ExternalProviderValue) => void;
  next: (data?: Partial<StoreValues['providerState']>) => void;
  back: (data?: Partial<StoreValues['providerState']>) => void;
}

export const useAddExternalOpenIdStore = create<Store>()(
  persist(
    (set, get) => ({
      ...addExternalOpenIdStoreDefaults,
      reset: () => set(addExternalOpenIdStoreDefaults),
      next: (data) => {
        const { provider, activeStep, providerState } = get();
        let targetStep = activeStep;
        const canDirectorySync = SUPPORTED_SYNC_PROVIDERS.has(provider);
        switch (activeStep) {
          case 'client-settings':
            if (canDirectorySync) {
              targetStep = AddExternalProviderStep.DirectorySync;
            } else {
              targetStep = AddExternalProviderStep.Validation;
            }
            break;
          case 'directory-sync':
            targetStep = AddExternalProviderStep.Validation;
            break;
        }
        set({
          activeStep: targetStep,
          providerState: { ...providerState, ...data },
        });
      },
      back: (data) => {
        const { provider, activeStep, providerState } = get();
        let targetStep = activeStep;
        const canDirectorySync = SUPPORTED_SYNC_PROVIDERS.has(provider);
        switch (activeStep) {
          case 'directory-sync':
            targetStep = AddExternalProviderStep.ClientSettings;
            break;
          case 'validation':
            if (canDirectorySync) {
              targetStep = AddExternalProviderStep.DirectorySync;
            } else {
              targetStep = AddExternalProviderStep.ClientSettings;
            }
            break;
        }
        set({
          activeStep: targetStep,
          providerState: { ...providerState, ...data },
        });
      },
      initialize: (provider) => {
        const initialProviderState = addExternalOpenIdStoreDefaults.providerState;
        initialProviderState.name = provider;
        if (provider !== ExternalProvider.Custom) {
          initialProviderState.display_name = externalProviderName[provider];
        }
        switch (provider) {
          case 'google':
            initialProviderState.base_url = googleProviderBaseUrl;
            break;
          case 'microsoft':
            break;
          case 'jumpCloud':
            initialProviderState.base_url = jumpcloudProviderBaseUrl;
            break;
          case 'okta':
            break;
        }
        set({
          activeStep: 'client-settings',
          provider,
          providerState: initialProviderState,
        });
      },
    }),
    {
      name: 'add-external-provider-store',
      storage: createJSONStorage(() => sessionStorage),
      partialize: (s) => omit(s, ['reset', 'initialize', 'next', 'back']),
    },
  ),
);
