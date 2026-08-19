import type { RefObject } from 'react';
import type { MfaFlowMethodValue } from '../../api/types';
import type { MenuItemProps } from '../../defguard-ui/components/Menu/types';

/** One ordered MFA step in the form state. */
export interface MfaConfigurationStepData {
  id: string | number;
  methods: MfaFlowMethodValue[];
}

/** MFA methods grouped for display in an add-method menu. */
export type MfaConfigurationMethodGroup = {
  header?: { text: string };
  items: MfaFlowMethodValue[];
};

/** Properties for an individual MFA step card. */
export type MfaConfigurationStepProps = {
  step: MfaConfigurationStepData;
  stepNumber: number;
  dragConstraints: RefObject<HTMLUListElement | null>;
  methodGroups: MfaConfigurationMethodGroup[];
  methodLabels: Record<MfaFlowMethodValue, string>;
  onDeleteStep: (id: MfaConfigurationStepData['id']) => void;
  onAddMethod: (
    stepId: MfaConfigurationStepData['id'],
    method: MfaFlowMethodValue,
  ) => void;
  onDeleteMethod: (
    stepId: MfaConfigurationStepData['id'],
    method: MfaFlowMethodValue,
  ) => void;
  buildOption: (method: MfaFlowMethodValue, onClick: () => void) => MenuItemProps;
};

export type MfaConfigurationProps = {
  steps: MfaConfigurationStepData[];
  onChange: (steps: MfaConfigurationStepData[]) => void;
  error?: string;
};
