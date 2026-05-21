import { Fragment } from 'react';
import type { ApiDevicePosture } from '../../shared/api/types';
import type { SelectionSectionCustomRender } from '../../shared/components/SelectionSection/type';
import { Checkbox } from '../../shared/defguard-ui/components/Checkbox/Checkbox';
import { Divider } from '../../shared/defguard-ui/components/Divider/Divider';
import { IconKind } from '../../shared/defguard-ui/components/Icon';
import { Icon } from '../../shared/defguard-ui/components/Icon/Icon';
import { TooltipContent } from '../../shared/defguard-ui/providers/tooltip/TooltipContent';
import { TooltipProvider } from '../../shared/defguard-ui/providers/tooltip/TooltipContext';
import { TooltipTrigger } from '../../shared/defguard-ui/providers/tooltip/TooltipTrigger';
import { ThemeVariable } from '../../shared/defguard-ui/types';
import { getPostureCheckAssignmentSummarySections } from './postureChecksSection';

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
          <TooltipProvider placement="right-start">
            <TooltipTrigger>
              <div
                className="posture-check-info-trigger"
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
            </TooltipTrigger>
            <TooltipContent className="posture-check-info-tooltip" variant="light">
              {sections.map((section, index) => (
                <Fragment key={section.label}>
                  {index > 0 && <Divider />}
                  <div className="posture-check-info-item">
                    <p className="label">{section.label}</p>
                    <div className="content">
                      {section.lines.map((line) => (
                        <p key={`${section.label}-${line}`}>{line}</p>
                      ))}
                    </div>
                  </div>
                </Fragment>
              ))}
            </TooltipContent>
          </TooltipProvider>
        }
      />
    </div>
  );
};
