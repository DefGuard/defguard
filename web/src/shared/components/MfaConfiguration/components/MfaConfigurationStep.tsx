import { Reorder, useDragControls } from 'motion/react';
import { m } from '../../../../paraglide/messages';
import { Divider } from '../../../defguard-ui/components/Divider/Divider';
import { FieldError } from '../../../defguard-ui/components/FieldError/FieldError';
import { Helper } from '../../../defguard-ui/components/Helper/Helper';
import { Icon } from '../../../defguard-ui/components/Icon';
import { SizedBox } from '../../../defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing, ThemeVariable } from '../../../defguard-ui/types';
import { mfaFlowMethodHints } from '../../../utils/mfaFlowSteps';
import type { MfaConfigurationStepProps } from '../types';
import { MfaMethodsMenu } from './MfaMethodsMenu';

export const MfaConfigurationStep = ({
  step,
  stepNumber,
  dragConstraints,
  isReordering,
  onReorderingChange,
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
    items: group.items.map((method) => ({
      ...buildOption(method, () => onAddMethod(step.id, method)),
      testId: `add-mfa-method-${stepNumber}-${method}`,
    })),
  }));
  const error = step.methods.length === 0 ? m.mfa_flow_method_required() : undefined;

  return (
    <Reorder.Item
      value={step}
      dragListener={false}
      dragControls={dragControls}
      dragConstraints={dragConstraints}
      dragElastic={false}
      layout="position"
      transition={isReordering ? undefined : { layout: { duration: 0 } }}
      onDragStart={() => onReorderingChange(true)}
      onDragTransitionEnd={() => onReorderingChange(false)}
      className="mfa-step-card"
      data-testid={`mfa-flow-step-${stepNumber}`}
    >
      <div className="top">
        <button
          type="button"
          className="drag-button"
          data-testid={`reorder-mfa-step-${stepNumber}`}
          aria-label={m.mfa_flow_step_reorder({ number: stepNumber })}
          onPointerDown={(event) => dragControls.start(event)}
        >
          <Icon icon="dnd" size={20} />
        </button>
        <p>{m.mfa_flow_step_title({ number: stepNumber })}</p>
        <button
          type="button"
          className="dispose-button"
          data-testid={`remove-mfa-step-${stepNumber}`}
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
                {mfaFlowMethodHints[method] && (
                  <span className="platform-label">
                    {mfaFlowMethodHints[method]?.platform}
                  </span>
                )}
                <button
                  type="button"
                  className="dispose-button"
                  data-testid={`remove-mfa-method-${stepNumber}-${method}`}
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
            testId={`add-mfa-method-${stepNumber}`}
            options={addMethodMenuOptions}
          />
          <Helper>
            <p>{m.mfa_flow_step_helper()}</p>
          </Helper>
        </div>
      )}
      {error && (
        <>
          <SizedBox height={ThemeSpacing.Lg} />
          <FieldError error={error} />
        </>
      )}
    </Reorder.Item>
  );
};
