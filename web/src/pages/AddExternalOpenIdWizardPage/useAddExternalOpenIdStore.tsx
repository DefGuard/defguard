import { omit } from 'lodash-es';
import { create } from 'zustand';
import { createJSONStorage, persist } from 'zustand/middleware';
import {
  type AddOpenIdProvider,
  DirectorySyncBehavior,
  DirectorySyncTarget,
  OpenIdProviderKind,
  type OpenIdProviderKindValue,
  OpenIdProviderUsernameHandling,
} from '../../shared/api/types';
import {
  externalProviderName,
  googleProviderBaseUrl,
  jumpcloudProviderBaseUrl,
  SUPPORTED_SYNC_PROVIDERS,
} from '../../shared/constants';
import { AddExternalProviderStep, type AddExternalProviderStepValue } from './types';

type ProviderState = AddOpenIdProvider & {
  microsoftTenantId?: string | null;
};

interface StoreValues {
  provider: OpenIdProviderKindValue;
  activeStep: AddExternalProviderStepValue;
  providerState: ProviderState;
  testResult: boolean | null;
  testMessage: string | null;
}

export const addExternalOpenIdStoreDefaults: StoreValues = {
  provider: OpenIdProviderKind.Custom,
  activeStep: 'client-settings',
  testResult: null,
  testMessage: null,
  providerState: {
    name: OpenIdProviderKind.Custom,
    base_url: '',
    kind: OpenIdProviderKind.Custom,
    client_id: '',
    client_secret: '',
    display_name: '',
    google_service_account_key: null,
    google_service_account_email: null,
    admin_email: '',
    directory_sync_enabled: false,
    directory_sync_interval: 600,
    directory_sync_user_behavior: DirectorySyncBehavior.Keep,
    directory_sync_admin_behavior: DirectorySyncBehavior.Keep,
    directory_sync_target: DirectorySyncTarget.All,
    okta_private_jwk: null,
    okta_dirsync_client_id: null,
    directory_sync_group_match: null,
    jumpcloud_api_key: null,
    prefetch_users: false,

    // Core settings
    create_account: false,
    username_handling: OpenIdProviderUsernameHandling.RemoveForbidden,

    microsoftTenantId: null,
  },
};

interface Store extends StoreValues {
  reset: () => void;
  initialize: (provider: OpenIdProviderKindValue) => void;
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
        initialProviderState.kind = provider;
        if (provider !== OpenIdProviderKind.Custom) {
          initialProviderState.display_name = externalProviderName[provider];
        }
        switch (provider) {
          case 'Google':
            initialProviderState.base_url = googleProviderBaseUrl;
            break;
          case 'Microsoft':
            break;
          case 'JumpCloud':
            initialProviderState.base_url = jumpcloudProviderBaseUrl;
            break;
          case 'Okta':
            break;
        }
        set({
          activeStep: 'client-settings',
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
