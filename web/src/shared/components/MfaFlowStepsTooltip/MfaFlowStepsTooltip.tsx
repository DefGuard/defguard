import './style.scss';

import type { Placement } from '@floating-ui/react';
import type { PropsWithChildren } from 'react';
import { m } from '../../../paraglide/messages';
import type { MfaFlowStepMethods } from '../../api/types';
import { mfaFlowMethodLabels } from '../../utils/mfaFlowSteps';
import { SummaryTooltip } from '../SummaryTooltip/SummaryTooltip';
import type { SummarySection } from '../SummaryTooltip/type';

type Props = {
  steps: MfaFlowStepMethods[];
  placement?: Placement;
};

export const MfaFlowStepsTooltip = ({
  steps,
  children,
  placement,
}: Props & PropsWithChildren) => {
  const sections: SummarySection[] = steps.map((step, index) => ({
    label: String(m.mfa_flow_step_title({ number: index + 1 })),
    lines: step.methods.map((method) => mfaFlowMethodLabels[method]),
  }));

  return (
    <SummaryTooltip
      sections={sections}
      className="mfa-flow-steps-tooltip"
      placement={placement}
    >
      {children}
    </SummaryTooltip>
  );
};
