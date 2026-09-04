import './style.scss';

import type { ApiDevicePosture } from '../../api/types';
import { Checkbox } from '../../defguard-ui/components/Checkbox/Checkbox';
import { IconKind } from '../../defguard-ui/components/Icon';
import { Icon } from '../../defguard-ui/components/Icon/Icon';
import { ThemeVariable } from '../../defguard-ui/types';
import { getPostureCheckAssignmentSummarySections } from '../../utils/postureCheckAssignmentSummary';
import type { SelectionSectionCustomRender } from '../SelectionSection/type';
import { SummaryTooltip } from '../SummaryTooltip/SummaryTooltip';

export const renderPostureCheckSelectionItem: SelectionSectionCustomRender<
  number,
  ApiDevicePosture
> = ({ active, onClick, option }) => {
  if (!option.meta) return null;

  const sections = getPostureCheckAssignmentSummarySections(option.meta);

  return (
    <div className="posture-check-selection-item">
      <Checkbox
        active={active}
        onClick={onClick}
        text={option.label}
        helperBlock={
          <SummaryTooltip sections={sections} className="posture-check-info-tooltip">
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
          </SummaryTooltip>
        }
      />
    </div>
  );
};
