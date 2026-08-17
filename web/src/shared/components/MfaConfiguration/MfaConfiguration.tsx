import './style.scss';
import { useQuery } from '@tanstack/react-query';
import { Reorder } from 'motion/react';
import { m } from '../../../paraglide/messages';
import { MfaFlowMethod, type MfaFlowMethodValue } from '../../api/types';
import { FieldError } from '../../defguard-ui/components/FieldError/FieldError';
import { useApp } from '../../hooks/useApp';
import { getLicenseInfoQueryOptions } from '../../query';
import { canUseBusinessFeature } from '../../utils/license';
import { MfaConfigurationStep } from './components/MfaConfigurationStep';
import { MfaMethodsMenu } from './components/MfaMethodsMenu';
import type {
  MfaConfigurationProps,
  MfaConfigurationStepData,
  MfaMethodGroup,
} from './types';

const availableMethods: MfaFlowMethodValue[] = [
  MfaFlowMethod.Email,
  MfaFlowMethod.MobileApprove,
  MfaFlowMethod.Totp,
  MfaFlowMethod.OpenId,
];

/** Configures ordered MFA steps and the methods accepted by each step. */
export const MfaConfiguration = ({ onChange, steps, error }: MfaConfigurationProps) => {
  const smtpEnabled = useApp((state) => state.appInfo.smtp_enabled);
  const { data: licenseInfo } = useQuery(getLicenseInfoQueryOptions);
  const businessAvailable =
    licenseInfo === undefined ? undefined : canUseBusinessFeature(licenseInfo).result;
  const methodLabels: Record<MfaFlowMethodValue, string> = {
    [MfaFlowMethod.MobileApprove]: m.mfa_flow_method_mobile_client(),
    [MfaFlowMethod.Totp]: m.mfa_flow_method_authenticator_app(),
    [MfaFlowMethod.OpenId]: m.mfa_flow_method_external_provider(),
    [MfaFlowMethod.Email]: m.mfa_flow_method_email_code(),
    [MfaFlowMethod.Biometric]: m.mfa_flow_method_biometric(),
  };
  /** Builds one selectable MFA method menu item. */
  const buildOption = (method: MfaFlowMethodValue, onClick: () => void) => {
    const smtpRequired = method === MfaFlowMethod.Email && !smtpEnabled;
    const businessRequired =
      method === MfaFlowMethod.OpenId && businessAvailable === false;
    const businessLoading =
      method === MfaFlowMethod.OpenId && businessAvailable === undefined;
    return {
      text: methodLabels[method],
      onClick,
      disabled: smtpRequired || businessRequired || businessLoading,
      disabledHelper: smtpRequired
        ? m.mfa_flow_method_smtp_required()
        : businessRequired
          ? m.mfa_flow_method_business_required()
          : undefined,
    };
  };
  /** Groups locked premium methods separately from methods available in the current plan. */
  const buildMethodGroups = (methods: MfaFlowMethodValue[]): MfaMethodGroup[] => {
    const externalIdLocked =
      businessAvailable === false && methods.includes(MfaFlowMethod.OpenId);
    if (!externalIdLocked) return [{ items: methods }];

    const planMethods = methods.filter((method) => method !== MfaFlowMethod.OpenId);
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
        items: [MfaFlowMethod.OpenId],
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
        />
      </div>
      <FieldError error={error} />
    </div>
  );
};
