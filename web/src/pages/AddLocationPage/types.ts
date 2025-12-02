export const AddLocationPageStep = {
  Start: 0,
  InternalVpnSettings: 1,
  NetworkSettings: 2,
  Mfa: 3,
  ServiceLocationSettings: 3,
  AccessControl: 4,
  Firewall: 5,
} as const;

export type AddLocationPageStepValue =
  (typeof AddLocationPageStep)[keyof typeof AddLocationPageStep];
