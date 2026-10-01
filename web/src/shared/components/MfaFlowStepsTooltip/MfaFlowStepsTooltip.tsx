import './style.scss';

import type { Placement } from '@floating-ui/react';
import type { PropsWithChildren } from 'react';
import { m } from '../../../paraglide/messages';
import type {
  MfaFlowStepMethods,
  MfaMethodAvailabilityReasonValue,
  MfaMethodAvailabilityResponse,
} from '../../api/types';
import { isPresent } from '../../defguard-ui/utils/isPresent';
import { mfaFlowMethodLabels } from '../../utils/mfaFlowSteps';
import { SummaryTooltip } from '../SummaryTooltip/SummaryTooltip';

type Props = {
  steps: MfaFlowStepMethods[];
  methodAvailability?: MfaMethodAvailabilityResponse[];
  unavailableReason?: MfaMethodAvailabilityReasonValue | null;
  placement?: Placement;
};

export const hasMfaFlowAvailabilityIssues = ({
  steps,
  methodAvailability = [],
  unavailableReason,
}: Pick<Props, 'steps' | 'methodAvailability' | 'unavailableReason'>) => {
  const availabilityByMethod = new Map(
    methodAvailability.map((item) => [item.method, item]),
  );

  return (
    isPresent(unavailableReason) ||
    steps.some((step) =>
      step.methods.some(
        (method) => availabilityByMethod.get(method)?.available === false,
      ),
    )
  );
};

export const MfaFlowStepsTooltip = ({
  steps,
  children,
  methodAvailability = [],
  placement,
  unavailableReason,
}: Props & PropsWithChildren) => {
  const availabilityByMethod = new Map(
    methodAvailability.map((item) => [item.method, item]),
  );
  const hasUnavailableMethod = steps.some((step) =>
    step.methods.some((method) => availabilityByMethod.get(method)?.available === false),
  );
  const hasUnavailableFlow = isPresent(unavailableReason);
  const sections = steps.map((step, index) => ({
    label: String(m.mfa_flow_step_title({ number: index + 1 })),
    lines: step.methods.map((method) => ({
      text: mfaFlowMethodLabels[method],
      warning: availabilityByMethod.get(method)?.available === false,
    })),
  }));

  return (
    <SummaryTooltip
      sections={sections}
      className="mfa-flow-steps-tooltip"
      placement={placement}
      footer={
        (hasUnavailableMethod || hasUnavailableFlow) && m.mfa_flow_unavailable_warning()
      }
    >
      {children}
    </SummaryTooltip>
  );
};
