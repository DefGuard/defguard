import { Reorder, useDragControls } from 'motion/react';
import { useMemo } from 'react';
import { m } from '../../../../paraglide/messages';
import { Divider } from '../../../defguard-ui/components/Divider/Divider';
import { Helper } from '../../../defguard-ui/components/Helper/Helper';
import { Icon } from '../../../defguard-ui/components/Icon';
import { ThemeSpacing, ThemeVariable } from '../../../defguard-ui/types';
import type { LocationMfaConfigurationStepProps } from '../types';
import { locationMfaMethodLabels } from '../types';
import { LocationMfaMethodsMenu } from './LocationMfaMethodsMenu';

export const LocationMfaConfigurationStep = ({
  step,
  methodGroups,
  onDeleteStep,
  onAddFactor,
  onDeleteFactor,
  buildOption,
}: LocationMfaConfigurationStepProps) => {
  const dragControls = useDragControls();

  const addFactorMenuOptions = useMemo(
    () =>
      methodGroups.map((group) => ({
        ...group,
        items: group.items.map((factor) =>
          buildOption(factor, () => onAddFactor(step.id, factor)),
        ),
      })),
    [methodGroups, buildOption, onAddFactor, step.id],
  );

  return (
    <Reorder.Item
      value={step}
      dragListener={false}
      dragControls={dragControls}
      layout="position"
      className="mfa-step-card"
      data-testid={`step-${step.id}`}
    >
      <div className="top">
        <button className="drag-button" onPointerDown={(e) => dragControls.start(e)}>
          <Icon icon="dnd" size={20} />
        </button>
        <p>{`Step ${step.order}`}</p>
        <button className="dispose-button" onClick={() => onDeleteStep(step.id)}>
          <Icon icon="delete" />
        </button>
      </div>
      <Divider spacing={ThemeSpacing.Lg} />
      <div className="factors">
        {step.factors.map((factor) => (
          <div key={factor} className="factor">
            <div className="track">
              <Icon icon="check-filled" staticColor={ThemeVariable.FgSuccess} />
              <p>{locationMfaMethodLabels[factor]}</p>
              <div className="right">
                {factor === 'biometry' && <p>{`Mobile only`}</p>}
                <button
                  className="dispose-button"
                  onClick={() => onDeleteFactor(step.id, factor)}
                >
                  <Icon icon="close" />
                </button>
              </div>
            </div>
          </div>
        ))}
      </div>
      {methodGroups.some((g) => g.items.length > 0) && (
        <div className="footer">
          <LocationMfaMethodsMenu
            kind="plain"
            label="+ Add factor"
            options={addFactorMenuOptions}
          />
          <Helper>
            <p>{m.test_placeholder_long()}</p>
          </Helper>
        </div>
      )}
    </Reorder.Item>
  );
};
