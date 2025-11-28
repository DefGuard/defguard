export const AddLocationPageStep = {
  Start: 0,
  VpnNetwork: 1,
  LocationAccess: 2,
  Firewall: 3,
  Mfa: 4,
} as const;

export type AddLocationPageStepValue =
  (typeof AddLocationPageStep)[keyof typeof AddLocationPageStep];
