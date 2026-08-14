import type { MenuItemProps } from '../../defguard-ui/components/Menu/types';

/** MFA methods supported by the flow editor and backend API. */
export const MfaMethod = {
  Totp: 'totp',
  Email: 'email',
  MobileClient: 'mobileapprove',
  OpenId: 'oidc',
} as const;

export type MfaMethodValue = (typeof MfaMethod)[keyof typeof MfaMethod];

/** One ordered MFA step in the form state. */
export interface MfaConfigurationStepData {
  id: string | number;
  methods: MfaMethodValue[];
}

/** Properties for an individual MFA step card. */
export type MfaConfigurationStepProps = {
  step: MfaConfigurationStepData;
  stepNumber: number;
  availableMethods: MfaMethodValue[];
  methodLabels: Record<MfaMethodValue, string>;
  onDeleteStep: (id: MfaConfigurationStepData['id']) => void;
  onAddMethod: (stepId: MfaConfigurationStepData['id'], method: MfaMethodValue) => void;
  onDeleteMethod: (
    stepId: MfaConfigurationStepData['id'],
    method: MfaMethodValue,
  ) => void;
  buildOption: (method: MfaMethodValue, onClick: () => void) => MenuItemProps;
};

/** Properties for the controlled MFA configuration editor. */
export type MfaConfigurationProps = {
  steps: MfaConfigurationStepData[];
  onChange: (steps: MfaConfigurationStepData[]) => void;
  error?: string;
};
