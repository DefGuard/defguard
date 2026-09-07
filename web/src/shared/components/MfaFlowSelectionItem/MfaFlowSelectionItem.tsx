import './style.scss';

import type { MfaFlowStepMethods } from '../../api/types';
import { IconKind } from '../../defguard-ui/components/Icon';
import { Icon } from '../../defguard-ui/components/Icon/Icon';
import { InteractiveBlock } from '../../defguard-ui/components/InteractiveBlock/InteractiveBlock';
import { ThemeVariable } from '../../defguard-ui/types';
import { MfaFlowStepsTooltip } from '../MfaFlowStepsTooltip/MfaFlowStepsTooltip';
import type { SelectionSectionCustomRender } from '../SelectionSection/type';

export type MfaFlowSelectionMeta = {
  steps: MfaFlowStepMethods[];
};

export const renderMfaFlowSelectionItem: SelectionSectionCustomRender<
  number,
  MfaFlowSelectionMeta
> = ({ active, onClick, option }) => {
  const steps = option.meta?.steps ?? [];

  return (
    <InteractiveBlock
      className="mfa-flow-selection-item"
      variant="radio"
      title={option.label}
      value={active}
      onClick={onClick}
      helperBlock={
        steps.length > 0 && (
          <MfaFlowStepsTooltip steps={steps}>
            <div
              className="summary-info-trigger"
              onClick={(event) => {
                event.stopPropagation();
              }}
            >
              <Icon
                icon={IconKind.InfoOutlined}
                size={20}
                staticColor={ThemeVariable.FgMuted}
              />
            </div>
          </MfaFlowStepsTooltip>
        )
      }
    />
  );
};
