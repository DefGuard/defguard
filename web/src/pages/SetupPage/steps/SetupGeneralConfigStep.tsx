import { useMutation } from '@tanstack/react-query';
import { useMemo } from 'react';
import z from 'zod';
import { useShallow } from 'zustand/react/shallow';
import api from '../../../shared/api/api';
import { WizardCard } from '../../../shared/components/wizard/WizardCard/WizardCard';
import { ModalControls } from '../../../shared/defguard-ui/components/ModalControls/ModalControls';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { Snackbar } from '../../../shared/defguard-ui/providers/snackbar/snackbar';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import { useAppForm } from '../../../shared/form';
import { formChangeLogic } from '../../../shared/formLogic';
import { SetupPageStep } from '../types';
import { useSetupWizardStore } from '../useSetupWizardStore';

type FormFields = StoreValues;

type StoreValues = {
  defguard_url: string;
  default_admin_group_name: string;
  default_authentication: number;
  default_mfa_code_lifetime: number;
};

export const SetupGeneralConfigStep = () => {
  const setActiveStep = useSetupWizardStore((s) => s.setActiveStep);
  const defaultValues = useSetupWizardStore(
    useShallow(
      (s): FormFields => ({
        defguard_url: s.defguard_url,
        default_admin_group_name: s.default_admin_group_name,
        default_authentication: s.default_authentication_period_days,
        default_mfa_code_lifetime: s.default_mfa_code_timeout_seconds,
      }),
    ),
  );

  const formSchema = useMemo(
    () =>
      z.object({
        defguard_url: z.url('Invalid URL').min(1, 'Defguard URL is required'),
        default_admin_group_name: z
          .string()
          .min(1, 'Default admin group name is required'),
        default_authentication: z
          .number()
          .min(1, 'Authentication period must be at least 1 day'),
        default_mfa_code_lifetime: z
          .number()
          .min(60, 'MFA code timeout must be at least 60 seconds'),
      }),
    [],
  );

  const { mutate, isPending } = useMutation({
    mutationFn: api.initial_setup.setGeneralConfig,
    meta: {
      invalidate: ['setupStatus'],
    },
    onSuccess: () => {
      setActiveStep(SetupPageStep.CertificateAuthority);
    },
    onError: (error) => {
      Snackbar.error('Failed to create admin user. Please try again.');
      console.error('Failed to create admin user:', error);
    },
  });

  const form = useAppForm({
    defaultValues,
    validationLogic: formChangeLogic,
    validators: {
      onSubmit: formSchema,
      onChange: formSchema,
    },
    onSubmit: ({ value }) => {
      useSetupWizardStore.setState({
        defguard_url: value.defguard_url,
        default_admin_group_name: value.default_admin_group_name,
        default_authentication_period_days: value.default_authentication,
        default_mfa_code_timeout_seconds: value.default_mfa_code_lifetime,
      });
      mutate({
        defguard_url: value.defguard_url,
        default_admin_group_name: value.default_admin_group_name,
        default_authentication: value.default_authentication,
        default_mfa_code_lifetime: value.default_mfa_code_lifetime,
        admin_username: useSetupWizardStore.getState().admin_username,
      });
    },
  });

  const handleNext = () => {
    form.handleSubmit();
  };

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
          <form.AppField name="defguard_url">
            {(field) => <field.FormInput required label="Defguard URL" type="text" />}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl} />
          <form.AppField name="default_admin_group_name">
            {(field) => (
              <field.FormInput required label="Default Admin Group Name" type="text" />
            )}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl} />
          <form.AppField name="default_authentication">
            {(field) => (
              <field.FormInput
                required
                label="Default Authentication Period (days)"
                type="number"
              />
            )}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl} />
          <form.AppField name="default_mfa_code_lifetime">
            {(field) => (
              <field.FormInput
                required
                label="Default MFA Code Timeout (seconds)"
                type="number"
              />
            )}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl} />
        </form.AppForm>
      </form>
      <ModalControls
        submitProps={{ text: 'Next', onClick: handleNext, loading: isPending }}
      />
    </WizardCard>
  );
};
