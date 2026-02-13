import { OpenIdProviderKind, type OpenIdProviderKindValue } from './api/types';

export const externalLink = {
  defguard: {
    pricing: 'https://defguard.net/pricing',
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

export const externalProviderName: Record<OpenIdProviderKindValue, string> = {
  Custom: 'Custom provider',
  Google: 'Google',
  JumpCloud: 'JumpCloud',
  Microsoft: 'Microsoft',
  Okta: 'Okta',
  Zitadel: 'Zitadel',
};

export const SUPPORTED_SYNC_PROVIDERS: Set<OpenIdProviderKindValue> = new Set([
  OpenIdProviderKind.Google,
  OpenIdProviderKind.Microsoft,
  OpenIdProviderKind.Okta,
  OpenIdProviderKind.JumpCloud,
]);

export const googleProviderBaseUrl = 'https://accounts.google.com';

export const jumpcloudProviderBaseUrl = 'https://oauth.id.jumpcloud.com';
