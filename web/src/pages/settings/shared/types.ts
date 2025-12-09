export const ExternalProvider = {
  Google: 'google',
  Microsoft: 'microsoft',
  Okta: 'okta',
  JumpCloud: 'jumpCloud',
  Zitadel: 'zitadel',
  Custom: 'custom',
} as const;

export type ExternalProviderValue =
  (typeof ExternalProvider)[keyof typeof ExternalProvider];
