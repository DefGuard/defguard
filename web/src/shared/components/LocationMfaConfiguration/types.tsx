export const LocationMfaMethod = {
  Totp: 'totp',
  Tpm: 'tpm',
  Email: 'email',
  Biometry: 'biometry',
  MobileConfirm: 'confirm_mobile',
  HardwareKey: 'hardware_key',
  OpenId: 'openid',
} as const;

export const locationMfaMethodLabels: Record<LocationMfaMethodValue, string> = {
  totp: 'Authenticator App',
  tpm: 'Hardware key (TPM 2.0)',
  email: 'Email Verification Code',
  biometry: 'Biometry',
  confirm_mobile: 'Defguard Mobile Client',
  hardware_key: 'Hardware Key',
  openid: 'External ID Provider',
};

export type LocationMfaMethodValue =
  (typeof LocationMfaMethod)[keyof typeof LocationMfaMethod];

export interface LocationMfaConfigurationStepData {
  id: string;
  order: number;
  factors: LocationMfaMethodValue[];
}

export interface LocationMfaConfigurationStepProps
  extends LocationMfaConfigurationStepData {
  onAdd: (factor: LocationMfaMethodValue) => void;
  onDelete: (step: LocationMfaConfigurationStepProps['id']) => void;
}

export type LocationMfaConfigurationProps = {
  steps: LocationMfaConfigurationStepData[];
  onChange: (steps: LocationMfaConfigurationStepData[]) => void;
  error?: string;
};
