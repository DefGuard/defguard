export const UserProfileTab = {
  Details: 'details',
  Devices: 'devices',
  AuthKeys: 'auth-keys',
  ApiTokens: 'api-tokens',
} as const;

export type UserProfileTabValue = (typeof UserProfileTab)[keyof typeof UserProfileTab];

export const ApiTokensTabAvailability = {
  Hidden: 'hidden',
  Loading: 'loading',
  Available: 'available',
  Unavailable: 'unavailable',
} as const;

export type ApiTokensTabAvailabilityValue =
  (typeof ApiTokensTabAvailability)[keyof typeof ApiTokensTabAvailability];
