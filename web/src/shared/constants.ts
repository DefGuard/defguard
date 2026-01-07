import {
  ExternalProvider,
  type ExternalProviderValue,
} from '../pages/settings/shared/types';

export const externalLink = {
  defguard: {
    download: 'https://defguard.net/download',
  },
  client: {
    desktop: {
      linux: {
        arch: 'https://aur.archlinux.org/packages/defguard-client',
      },
    },
    mobile: {
      apple: 'https://apps.apple.com/us/app/defguard-vpn-client/id6748068630',
      google: 'https://play.google.com/store/apps/details?id=net.defguard.mobile',
    },
  },
} as const;

export const externalProviderName: Record<ExternalProviderValue, string> = {
  custom: 'Custom provider',
  google: 'Google',
  jumpCloud: 'JumpCloud',
  microsoft: 'Microsoft',
  okta: 'Okta',
  zitadel: 'Zitadel',
};

export const SUPPORTED_SYNC_PROVIDERS: Set<ExternalProviderValue> = new Set([
  ExternalProvider.Google,
  ExternalProvider.Microsoft,
  ExternalProvider.Okta,
  ExternalProvider.JumpCloud,
]);

export const googleProviderBaseUrl = 'https://accounts.google.com';

export const jumpcloudProviderBaseUrl = 'https://oauth.id.jumpcloud.com';
