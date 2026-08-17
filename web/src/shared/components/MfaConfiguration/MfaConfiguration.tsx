import './style.scss';
import { useQuery } from '@tanstack/react-query';
import { Reorder } from 'motion/react';
import { m } from '../../../paraglide/messages';
import {
  MfaFlowMethod,
  type MfaFlowMethodValue,
  MfaMethodAvailabilityReason,
  type MfaMethodAvailabilityReasonValue,
} from '../../api/types';
import { FieldError } from '../../defguard-ui/components/FieldError/FieldError';
import { getMfaMethodAvailabilityQueryOptions } from '../../query';
import { MfaConfigurationStep } from './components/MfaConfigurationStep';
import { MfaMethodsMenu } from './components/MfaMethodsMenu';
import type {
  MfaConfigurationProps,
  MfaConfigurationStepData,
  MfaMethodGroup,
} from './types';

/** Configures ordered MFA steps and the methods accepted by each step. */
export const MfaConfiguration = ({ onChange, steps, error }: MfaConfigurationProps) => {
  const { data: methodAvailability } = useQuery(getMfaMethodAvailabilityQueryOptions);
  const availableMethods = methodAvailability?.map(({ method }) => method) ?? [];
  const methodLabels: Record<MfaFlowMethodValue, string> = {
    [MfaFlowMethod.MobileApprove]: m.mfa_flow_method_mobile_client(),
    [MfaFlowMethod.Totp]: m.mfa_flow_method_authenticator_app(),
    [MfaFlowMethod.OpenId]: m.mfa_flow_method_external_provider(),
    [MfaFlowMethod.Email]: m.mfa_flow_method_email_code(),
    [MfaFlowMethod.Biometric]: m.mfa_flow_method_biometric(),
  };
  /** Maps a backend availability reason to menu guidance. */
  const getDisabledHelper = (reason: MfaMethodAvailabilityReasonValue) => {
    switch (reason) {
      case MfaMethodAvailabilityReason.Licensed:
        return m.mfa_flow_method_business_required();
      case MfaMethodAvailabilityReason.SmtpNotConfigured:
        return m.mfa_flow_method_smtp_required();
      case MfaMethodAvailabilityReason.OidcProviderMissing:
        return m.mfa_flow_error_oidc_provider_missing();
      case MfaMethodAvailabilityReason.Available:
        return undefined;
    }
  };
  /** Builds one selectable MFA method menu item. */
  const buildOption = (method: MfaFlowMethodValue, onClick: () => void) => {
    const availability = methodAvailability?.find((item) => item.method === method);
    return {
      text: methodLabels[method],
      onClick,
      disabled: availability?.available !== true,
      disabledHelper: availability ? getDisabledHelper(availability.reason) : undefined,
    };
  };
  /** Groups locked premium methods separately from methods available in the current plan. */
  const buildMethodGroups = (methods: MfaFlowMethodValue[]): MfaMethodGroup[] => {
    const licensedMethods = methods.filter(
      (method) =>
        methodAvailability?.find((item) => item.method === method)?.reason ===
        MfaMethodAvailabilityReason.Licensed,
    );
    if (licensedMethods.length === 0) return [{ items: methods }];

    const planMethods = methods.filter((method) => !licensedMethods.includes(method));
    return [
      ...(planMethods.length > 0
        ? [
            {
              header: { text: m.mfa_flow_methods_available_in_plan() },
              items: planMethods,
            },
          ]
        : []),
      {
        header: { text: m.mfa_flow_methods_available_in_higher_plans() },
        items: licensedMethods,
      },
    ];
  };
  const addStepMenuOptions = buildMethodGroups(availableMethods).map((group) => ({
    ...group,
    items: group.items.map((method) =>
      buildOption(method, () => {
        onChange([...steps, { id: crypto.randomUUID(), methods: [method] }]);
      }),
    ),
  }));

  /** Removes a complete MFA step. */
  const deleteStep = (id: MfaConfigurationStepData['id']) => {
    onChange(steps.filter((step) => step.id !== id));
  };
  /** Adds a method to an existing MFA step. */
  const addMethod = (
    stepId: MfaConfigurationStepData['id'],
    method: MfaFlowMethodValue,
  ) => {
    onChange(
      steps.map((step) =>
        step.id === stepId ? { ...step, methods: [...step.methods, method] } : step,
      ),
    );
  };
  /** Removes a method and drops the step when it becomes empty. */
  const deleteMethod = (
    stepId: MfaConfigurationStepData['id'],
    method: MfaFlowMethodValue,
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
            methodGroups={buildMethodGroups(
              availableMethods.filter((method) => !step.methods.includes(method)),
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
          disabled={methodAvailability === undefined}
        />
      </div>
      <FieldError error={error} />
    </div>
  );
};
