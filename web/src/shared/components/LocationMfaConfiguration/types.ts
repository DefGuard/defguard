import type { MenuItemProps } from '../../defguard-ui/components/Menu/types';

export const LocationMfaMethod = {
  Totp: 'totp',
  Tpm: 'tpm',
  Email: 'email',
  Biometry: 'biometry',
  MobileConfirm: 'confirm_mobile',
  Fido2: 'fido2',
  OpenId: 'openid',
} as const;

export const locationMfaMethodLabels: Record<LocationMfaMethodValue, string> = {
  totp: 'Authenticator App',
  tpm: 'Hardware key (TPM 2.0)',
  email: 'Email Verification Code',
  biometry: 'Biometry',
  confirm_mobile: 'Defguard Mobile Client',
  fido2: 'FIDO2 Security Key',
  openid: 'External ID Provider',
};

export type LocationMfaMethodValue =
  (typeof LocationMfaMethod)[keyof typeof LocationMfaMethod];

export interface LocationMfaConfigurationStepData {
  id: string;
  order: number;
  factors: LocationMfaMethodValue[];
}

export type LocationMfaMethodGroup = {
  header?: { text: string };
  items: LocationMfaMethodValue[];
};

export type LocationMfaConfigurationStepProps = {
  step: LocationMfaConfigurationStepData;
  methodGroups: LocationMfaMethodGroup[];
  onDeleteStep: (id: string) => void;
  onAddFactor: (stepId: string, factor: LocationMfaMethodValue) => void;
  onDeleteFactor: (stepId: string, factor: LocationMfaMethodValue) => void;
  buildOption: (method: LocationMfaMethodValue, onClick: () => void) => MenuItemProps;
};

export type LocationMfaConfigurationProps = {
  steps: LocationMfaConfigurationStepData[];
  onChange: (steps: LocationMfaConfigurationStepData[]) => void;
  error?: string;
};
