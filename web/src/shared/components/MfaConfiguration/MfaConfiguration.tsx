import './style.scss';
import { useQuery } from '@tanstack/react-query';
import { Reorder } from 'motion/react';
import { useCallback, useRef } from 'react';
import { m } from '../../../paraglide/messages';
import {
  MfaFlowMethod,
  type MfaFlowMethodValue,
  MfaMethodAvailabilityReason,
  type MfaMethodAvailabilityReasonValue,
} from '../../api/types';
import type { ButtonProps } from '../../defguard-ui/components/Button/types';
import { FieldError } from '../../defguard-ui/components/FieldError/FieldError';
import { InfoBanner } from '../../defguard-ui/components/InfoBanner/InfoBanner';
import {
  getLicenseInfoQueryOptions,
  getMfaMethodAvailabilityQueryOptions,
} from '../../query';
import { canUseBusinessFeature } from '../../utils/license';
import { MfaConfigurationStep } from './components/MfaConfigurationStep';
import { MfaMethodsMenu } from './components/MfaMethodsMenu';
import type {
  MfaConfigurationMethodGroup,
  MfaConfigurationProps,
  MfaConfigurationStepData,
} from './types';

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

const methodLabels: Record<MfaFlowMethodValue, string> = {
  [MfaFlowMethod.MobileApprove]: m.mfa_flow_method_mobile_client(),
  [MfaFlowMethod.Totp]: m.mfa_flow_method_authenticator_app(),
  [MfaFlowMethod.OpenId]: m.mfa_flow_method_external_provider(),
  [MfaFlowMethod.Email]: m.mfa_flow_method_email_code(),
  [MfaFlowMethod.Biometric]: m.mfa_flow_method_biometric(),
};

export const MfaConfiguration = ({ onChange, steps, error }: MfaConfigurationProps) => {
  const stepsTrackRef = useRef<HTMLUListElement>(null);
  const { data: methodData } = useQuery(getMfaMethodAvailabilityQueryOptions);
  const { data: licenseInfo } = useQuery(getLicenseInfoQueryOptions);
  const methodAvailability = methodData?.methodAvailability;
  const methods = methodData?.methods ?? [];
  const businessLicenseCheck =
    licenseInfo === undefined ? undefined : canUseBusinessFeature(licenseInfo);
  const additionalStepRequiresBusiness =
    steps.length > 0 && businessLicenseCheck?.result === false;

  const buildOption = useCallback(
    (method: MfaFlowMethodValue, onClick: () => void) => {
      const availability = methodAvailability?.find((item) => item.method === method);
      return {
        text: methodLabels[method],
        onClick,
        disabled: availability?.available !== true,
        disabledHelper: availability ? getDisabledHelper(availability.reason) : undefined,
      };
    },
    [methodAvailability],
  );
  const buildMethodGroups = (
    methods: MfaFlowMethodValue[],
  ): MfaConfigurationMethodGroup[] => {
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

  const addStepMenuOptions = buildMethodGroups(methods).map((group) => ({
    ...group,
    items: group.items.map((method) =>
      buildOption(method, () => {
        onChange([...steps, { id: crypto.randomUUID(), methods: [method] }]);
      }),
    ),
  }));
  const addStepButtonProps: Omit<ButtonProps, 'ref'> = {
    type: 'button',
    variant: 'outlined',
    iconRight: 'arrow-small',
    iconRightRotation: 'down',
    text: m.mfa_flow_step_add(),
    disabled:
      methodAvailability === undefined ||
      (steps.length > 0 && businessLicenseCheck === undefined),
  };

  const deleteStep = useCallback(
    (id: MfaConfigurationStepData['id']) => {
      onChange(steps.filter((step) => step.id !== id));
    },
    [onChange, steps],
  );

  const addMethod = useCallback(
    (stepId: MfaConfigurationStepData['id'], method: MfaFlowMethodValue) => {
      onChange(
        steps.map((step) =>
          step.id === stepId ? { ...step, methods: [...step.methods, method] } : step,
        ),
      );
    },
    [onChange, steps],
  );

  const deleteMethod = useCallback(
    (stepId: MfaConfigurationStepData['id'], method: MfaFlowMethodValue) => {
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
    },
    [deleteStep, onChange, steps],
  );

  return (
    <div className="mfa-configuration">
      <Reorder.Group
        ref={stepsTrackRef}
        axis="y"
        values={steps}
        onReorder={onChange}
        className="steps-track"
      >
        {steps.map((step, index) => (
          <MfaConfigurationStep
            key={step.id}
            step={step}
            stepNumber={index + 1}
            dragConstraints={stepsTrackRef}
            methodGroups={buildMethodGroups(
              methods.filter((method) => !step.methods.includes(method)),
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
        {additionalStepRequiresBusiness ? (
          <InfoBanner
            icon="check-circle"
            variant="action"
            text={m.mfa_flow_additional_steps_business_required()}
          />
        ) : (
          <MfaMethodsMenu
            {...addStepButtonProps}
            kind="button"
            options={addStepMenuOptions}
          />
        )}
      </div>
      <FieldError error={error} />
    </div>
  );
};
