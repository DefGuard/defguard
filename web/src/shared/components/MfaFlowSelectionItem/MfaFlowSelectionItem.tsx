import './style.scss';

import type {
  MfaFlowStepMethods,
  MfaMethodAvailabilityReasonValue,
  MfaMethodAvailabilityResponse,
} from '../../api/types';
import { IconKind } from '../../defguard-ui/components/Icon';
import { Icon } from '../../defguard-ui/components/Icon/Icon';
import { InteractiveBlock } from '../../defguard-ui/components/InteractiveBlock/InteractiveBlock';
import { ThemeVariable } from '../../defguard-ui/types';
import {
  hasMfaFlowAvailabilityIssues,
  MfaFlowStepsTooltip,
} from '../MfaFlowStepsTooltip/MfaFlowStepsTooltip';
import type { SelectionSectionCustomRender } from '../SelectionSection/type';

export type MfaFlowSelectionMeta = {
  steps: MfaFlowStepMethods[];
  methodAvailability?: MfaMethodAvailabilityResponse[];
  unavailableReason?: MfaMethodAvailabilityReasonValue | null;
};

export const renderMfaFlowSelectionItem: SelectionSectionCustomRender<
  number,
  MfaFlowSelectionMeta
> = ({ active, onClick, option }) => {
  const steps = option.meta?.steps ?? [];
  const hasAvailabilityIssues = hasMfaFlowAvailabilityIssues({
    steps,
    methodAvailability: option.meta?.methodAvailability,
    unavailableReason: option.meta?.unavailableReason,
  });

  return (
    <InteractiveBlock
      className="mfa-flow-selection-item"
      variant="radio"
      title={option.label}
      value={active}
      onClick={onClick}
      helperBlock={
        steps.length > 0 && (
          <MfaFlowStepsTooltip
            steps={steps}
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
                icon={IconKind.InfoOutlined}
                size={20}
                staticColor={
                  hasAvailabilityIssues
                    ? ThemeVariable.FgAttention
                    : ThemeVariable.FgMuted
                }
              />
            </div>
          </MfaFlowStepsTooltip>
        )
      }
    />
  );
};
