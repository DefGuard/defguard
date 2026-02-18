import { useMemo } from 'react';
import z from 'zod';
import { useShallow } from 'zustand/react/shallow';
import { m } from '../../../paraglide/messages';
import { WizardCard } from '../../../shared/components/wizard/WizardCard/WizardCard';
import { ModalControls } from '../../../shared/defguard-ui/components/ModalControls/ModalControls';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import { useAppForm } from '../../../shared/form';
import { formChangeLogic } from '../../../shared/formLogic';
import { GatewaySetupStep } from '../types';
import { useGatewayWizardStore } from '../useGatewayWizardStore';
import './style.scss';
import { validateIpOrDomain } from '../../../shared/validators';

type FormFields = StoreValues;

type StoreValues = {
  common_name: string;
  ip_or_domain: string;
  grpc_port: number;
};

export const SetupGatewayComponentStep = () => {
  const setActiveStep = useGatewayWizardStore((s) => s.setActiveStep);

  const defaultValues = useGatewayWizardStore(
    useShallow(
      (s): FormFields => ({
        common_name: s.common_name,
        ip_or_domain: s.ip_or_domain,
        grpc_port: s.grpc_port,
      }),
    ),
  );

  const handleNext = () => {
    form.handleSubmit();
  };

  const handleBack = () => {
    useGatewayWizardStore.setState({
      activeStep: GatewaySetupStep.DeployGateway,
    });
  };

  const formSchema = useMemo(
    () =>
      z.object({
        common_name: z
          .string()
          .min(1, m.edge_setup_component_error_common_name_required()),
        ip_or_domain: z
          .string()
          .min(1, m.edge_setup_component_error_ip_or_domain_required())
          .refine((val) => validateIpOrDomain(val, false, true)),
        grpc_port: z
          .number()
          .min(1, m.edge_setup_component_error_grpc_port_required())
          .max(65535, m.edge_setup_component_error_grpc_port_max()),
      }),
    [],
  );

  const form = useAppForm({
    defaultValues,
    validationLogic: formChangeLogic,
    validators: {
      onSubmit: formSchema,
      onChange: formSchema,
    },
    onSubmit: ({ value }) => {
      useGatewayWizardStore.setState({
        ...value,
      });
      setActiveStep(GatewaySetupStep.GatewayAdoption);
    },
  });

  return (
    <WizardCard>
      <form
        onSubmit={(e) => {
          e.stopPropagation();
          e.preventDefault();
          form.handleSubmit();
        }}
      >
        <form.AppForm>
          <form.AppField name="common_name">
            {(field) => (
              <field.FormInput
                required
                label={m.gateway_setup_component_label_common_name()}
                type="text"
              />
            )}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl} />
          <form.AppField name="ip_or_domain">
            {(field) => (
              <field.FormInput
                required
                label={m.gateway_setup_component_label_ip_or_domain()}
                type="text"
              />
            )}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl} />
          <form.AppField name="grpc_port">
            {(field) => (
              <field.FormInput
                required
                label={m.gateway_setup_component_label_grpc_port()}
                type="number"
              />
            )}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl} />
        </form.AppForm>
      </form>
      <ModalControls
        cancelProps={{
          text: m.gateway_setup_component_controls_back(),
          onClick: handleBack,
          variant: 'outlined',
        }}
        submitProps={{
          text: m.gateway_setup_component_controls_submit(),
          onClick: handleNext,
          type: 'submit',
        }}
      />
    </WizardCard>
  );
};
