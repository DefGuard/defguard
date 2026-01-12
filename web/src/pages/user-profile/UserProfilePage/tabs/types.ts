export const UserProfileTab = {
  Details: 'details',
  Devices: 'devices',
  AuthKeys: 'auth-keys',
  ApiTokens: 'api-tokens',
} as const;

export type UserProfileTabValue = (typeof UserProfileTab)[keyof typeof UserProfileTab];
