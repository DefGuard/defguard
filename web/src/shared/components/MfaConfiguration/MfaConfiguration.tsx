import './style.scss';
import { Reorder } from 'motion/react';
import { m } from '../../../paraglide/messages';
import { FieldError } from '../../defguard-ui/components/FieldError/FieldError';
import { MfaConfigurationStep } from './components/MfaConfigurationStep';
import { MfaMethodsMenu } from './components/MfaMethodsMenu';
import type {
  MfaConfigurationProps,
  MfaConfigurationStepData,
  MfaMethodValue,
} from './types';
import { MfaMethod } from './types';

const availableMethods: MfaMethodValue[] = [
  MfaMethod.MobileClient,
  MfaMethod.Totp,
  MfaMethod.OpenId,
  MfaMethod.Email,
];

/** Configures ordered MFA steps and the methods accepted by each step. */
export const MfaConfiguration = ({ onChange, steps, error }: MfaConfigurationProps) => {
  const methodLabels: Record<MfaMethodValue, string> = {
    [MfaMethod.MobileClient]: m.mfa_flow_method_mobile_client(),
    [MfaMethod.Totp]: m.mfa_flow_method_authenticator_app(),
    [MfaMethod.OpenId]: m.mfa_flow_method_external_provider(),
    [MfaMethod.Email]: m.mfa_flow_method_email_code(),
  };
  /** Builds one selectable MFA method menu item. */
  const buildOption = (method: MfaMethodValue, onClick: () => void) => ({
    text: methodLabels[method],
    onClick,
  });
  const addStepMenuOptions = [
    {
      items: availableMethods.map((method) =>
        buildOption(method, () => {
          onChange([...steps, { id: crypto.randomUUID(), methods: [method] }]);
        }),
      ),
    },
  ];

  /** Removes a complete MFA step. */
  const deleteStep = (id: MfaConfigurationStepData['id']) => {
    onChange(steps.filter((step) => step.id !== id));
  };
  /** Adds a method to an existing MFA step. */
  const addMethod = (stepId: MfaConfigurationStepData['id'], method: MfaMethodValue) => {
    onChange(
      steps.map((step) =>
        step.id === stepId ? { ...step, methods: [...step.methods, method] } : step,
      ),
    );
  };
  /** Removes a method and drops the step when it becomes empty. */
  const deleteMethod = (
    stepId: MfaConfigurationStepData['id'],
    method: MfaMethodValue,
  ) => {
    const step = steps.find((item) => item.id === stepId);
    if (!step) return;

    if (step.methods.length === 1) {
      deleteStep(stepId);
      return;
    }

    onChange(
      steps.map((item) =>
        item.id === stepId
          ? {
              ...item,
              methods: item.methods.filter((itemMethod) => itemMethod !== method),
            }
          : item,
      ),
    );
  };

  return (
    <div className="mfa-configuration">
      <Reorder.Group axis="y" values={steps} onReorder={onChange} className="steps-track">
        {steps.map((step, index) => (
          <MfaConfigurationStep
            key={step.id}
            step={step}
            stepNumber={index + 1}
            availableMethods={availableMethods.filter(
              (method) => !step.methods.includes(method),
            )}
            methodLabels={methodLabels}
            onDeleteStep={deleteStep}
            onAddMethod={addMethod}
            onDeleteMethod={deleteMethod}
            buildOption={buildOption}
          />
        ))}
      </Reorder.Group>
      <div className="actions">
        <MfaMethodsMenu
          kind="button"
          type="button"
          variant="outlined"
          iconRight="arrow-small"
          iconRightRotation="down"
          text={m.mfa_flow_step_add()}
          options={addStepMenuOptions}
        />
      </div>
      <FieldError error={error} />
    </div>
  );
};
