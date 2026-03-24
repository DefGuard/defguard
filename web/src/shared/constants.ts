import { OpenIdProviderKind, type OpenIdProviderKindValue } from './api/types';

export const externalLink = {
  defguard: {
    docs: 'https://docs.defguard.net',
    pricing: 'https://defguard.net/pricing',
    download: 'https://defguard.net/download',
    sales: 'mailto:sales@defguard.net',
    support: 'https://defguard.net/support/',
    scheduleCall: 'https://calendly.com/defguard',
  },
  github: {
    bugReport: 'https://github.com/DefGuard/defguard/issues/new?template=bug_report.md',
    featureRequest:
      'https://github.com/DefGuard/defguard/issues/new?template=feature_request.md',
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
