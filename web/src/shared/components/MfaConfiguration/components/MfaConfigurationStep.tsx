import { Reorder, useDragControls } from 'motion/react';
import { m } from '../../../../paraglide/messages';
import { Divider } from '../../../defguard-ui/components/Divider/Divider';
import { Helper } from '../../../defguard-ui/components/Helper/Helper';
import { Icon } from '../../../defguard-ui/components/Icon';
import { ThemeSpacing, ThemeVariable } from '../../../defguard-ui/types';
import type { MfaConfigurationStepProps } from '../types';
import { MfaMethodsMenu } from './MfaMethodsMenu';

export const MfaConfigurationStep = ({
  step,
  stepNumber,
  dragConstraints,
  methodGroups,
  methodLabels,
  onDeleteStep,
  onAddMethod,
  onDeleteMethod,
  buildOption,
}: MfaConfigurationStepProps) => {
  const dragControls = useDragControls();
  const addMethodMenuOptions = methodGroups.map((group) => ({
    ...group,
    items: group.items.map((method) =>
      buildOption(method, () => onAddMethod(step.id, method)),
    ),
  }));

  return (
    <Reorder.Item
      value={step}
      dragListener={false}
      dragControls={dragControls}
      dragConstraints={dragConstraints}
      dragElastic={false}
      layout="position"
      className="mfa-step-card"
      data-testid={`step-${step.id}`}
    >
      <div className="top">
        <button
          type="button"
          className="drag-button"
          aria-label={m.mfa_flow_step_reorder({ number: stepNumber })}
          onPointerDown={(event) => dragControls.start(event)}
        >
          <Icon icon="dnd" size={20} />
        </button>
        <p>{m.mfa_flow_step_title({ number: stepNumber })}</p>
        <button
          type="button"
          className="dispose-button"
          aria-label={m.mfa_flow_step_remove({ number: stepNumber })}
          onClick={() => onDeleteStep(step.id)}
        >
          <Icon icon="delete" />
        </button>
      </div>
      <Divider spacing={ThemeSpacing.Lg} />
      <div className="methods">
        {step.methods.map((method) => (
          <div key={method} className="method">
            <div className="track">
              <Icon icon="check-filled" size={16} staticColor={ThemeVariable.FgSuccess} />
              <p>{methodLabels[method]}</p>
              <div className="right">
                <button
                  type="button"
                  className="dispose-button"
                  aria-label={m.mfa_flow_method_remove({ method: methodLabels[method] })}
                  onClick={() => onDeleteMethod(step.id, method)}
                >
                  <Icon icon="close" />
                </button>
              </div>
            </div>
          </div>
        ))}
      </div>
      {methodGroups.some((group) => group.items.length > 0) && (
        <div className="footer">
          <MfaMethodsMenu
            kind="plain"
            label={m.mfa_flow_method_add()}
            options={addMethodMenuOptions}
          />
          <Helper>
            <p>{m.mfa_flow_step_helper()}</p>
          </Helper>
        </div>
      )}
    </Reorder.Item>
  );
};
