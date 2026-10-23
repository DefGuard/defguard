import './style.scss';

import { useHover } from '@uidotdev/usehooks';
import clsx from 'clsx';
import type {
  MfaFlowStepMethods,
  MfaMethodAvailabilityReasonValue,
  MfaMethodAvailabilityResponse,
} from '../../api/types';
import { IconKind } from '../../defguard-ui/components/Icon';
import { Icon } from '../../defguard-ui/components/Icon/Icon';
import { RadioIndicator } from '../../defguard-ui/components/RadioIndicator/RadioIndicator';
import { ThemeVariable } from '../../defguard-ui/types';
import {
  hasMfaFlowAvailabilityIssues,
  MfaFlowStepsTooltip,
} from '../MfaFlowStepsTooltip/MfaFlowStepsTooltip';
import type {
  SelectionSectionCustomItemRenderProps,
  SelectionSectionCustomRender,
} from '../SelectionSection/type';

export type MfaFlowSelectionMeta = {
  steps: MfaFlowStepMethods[];
  methodAvailability?: MfaMethodAvailabilityResponse[];
  unavailableReason?: MfaMethodAvailabilityReasonValue | null;
};

const MfaFlowSelectionItem = ({
  active,
  onClick,
  option,
}: SelectionSectionCustomItemRenderProps<number, MfaFlowSelectionMeta>) => {
  const [ref, hover] = useHover<HTMLDivElement>();
  const steps = option.meta?.steps ?? [];
  const hasAvailabilityIssues = hasMfaFlowAvailabilityIssues({
    steps,
    methodAvailability: option.meta?.methodAvailability,
    unavailableReason: option.meta?.unavailableReason,
  });

  return (
    <div
      ref={ref}
      className={clsx('mfa-flow-selection-item', { active })}
      role="button"
      tabIndex={0}
      data-active={active}
      onClick={onClick}
    >
      <RadioIndicator active={active} hover={hover} />
      <p className="label">{option.label}</p>
      {steps.length > 0 && (
        <MfaFlowStepsTooltip
          steps={steps}
          placement="top"
          methodAvailability={option.meta?.methodAvailability}
          unavailableReason={option.meta?.unavailableReason}
        >
          <div
            className="summary-info-trigger"
            onClick={(event) => {
              event.stopPropagation();
            }}
          >
            <Icon
              icon={
                hasAvailabilityIssues ? IconKind.AttentionFilled : IconKind.InfoOutlined
              }
              size={20}
              staticColor={
                hasAvailabilityIssues ? ThemeVariable.FgAttention : ThemeVariable.FgMuted
              }
            />
          </div>
        </MfaFlowStepsTooltip>
      )}
    </div>
  );
};

export const renderMfaFlowSelectionItem: SelectionSectionCustomRender<
  number,
  MfaFlowSelectionMeta
> = (props) => <MfaFlowSelectionItem {...props} />;
