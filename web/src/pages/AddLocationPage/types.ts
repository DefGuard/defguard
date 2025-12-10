export const AddLocationPageStep = {
  Start: 'start',
  InternalVpnSettings: 'internalVpnSettings',
  NetworkSettings: 'networkSettings',
  Mfa: 'mfa',
  ServiceLocationSettings: 'serviceLocationSettings',
  AccessControl: 'accessControl',
  Firewall: 'firewall',
} as const;

export type AddLocationPageStepValue =
  (typeof AddLocationPageStep)[keyof typeof AddLocationPageStep];
