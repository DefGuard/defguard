export const LicenseModalSideImageVariant = {
  Expired: 'expired',
  Limit: 'limit',
  Business: 'business',
  Enterprise: 'enterprise',
  Reactivated: 'reactivated',
} as const;

export type LicenseModalSideImageVariantValue =
  (typeof LicenseModalSideImageVariant)[keyof typeof LicenseModalSideImageVariant];
