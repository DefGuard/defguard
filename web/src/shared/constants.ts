import { OpenIdProviderKind, type OpenIdProviderKindValue } from './api/types';

export const externalLink = {
  defguard: {
    docs: 'https://docs.defguard.net',
    pricing: 'https://defguard.net/pricing',
    download: 'https://defguard.net/download',
    sales: 'https://defguard.net/contact/',
    scheduleCall:
      'https://docs.google.com/forms/d/e/1FAIpQLSdKr1NXH1DlQuAF5oQWvT7Zri5yPQ3txvwz3qgtb1n9FtKTgw/viewform',
  },
  github: {
    bugReport: 'https://github.com/DefGuard/defguard/issues/new?template=02-bug.yml',
    featureRequest:
      'https://github.com/DefGuard/defguard/issues/new?template=01-feature-request.yml',
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

export const supportedSyncProviders: Set<OpenIdProviderKindValue> = new Set([
  OpenIdProviderKind.Google,
  OpenIdProviderKind.Microsoft,
  OpenIdProviderKind.Okta,
  OpenIdProviderKind.JumpCloud,
]);

export const googleProviderBaseUrl = 'https://accounts.google.com';

export const jumpcloudProviderBaseUrl = 'https://oauth.id.jumpcloud.com';

export const licenseGracePeriodDays = 14;

export const edgeDefaultGrpcPort = 50051;

export const gatewayDefaultGrpcPort = 50066;

export const DISMISSED_UPDATE_KEY = 'dismissed-update-version';
