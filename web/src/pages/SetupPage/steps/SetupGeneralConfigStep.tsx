import { useMutation } from '@tanstack/react-query';
import { useMemo } from 'react';
import z from 'zod';
import { useShallow } from 'zustand/react/shallow';
import { m } from '../../../paraglide/messages';
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
  public_proxy_url: string;
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
        public_proxy_url: s.public_proxy_url,
      }),
    ),
  );

  const formSchema = useMemo(
    () =>
      z.object({
        defguard_url: z
          .url(m.initial_setup_general_config_error_invalid_url())
          .min(1, m.initial_setup_general_config_error_defguard_url_required()),
        default_admin_group_name: z
          .string()
          .min(1, m.initial_setup_general_config_error_admin_group_required()),
        default_authentication: z
          .number()
          .min(1, m.initial_setup_general_config_error_auth_period_min()),
        default_mfa_code_lifetime: z
          .number()
          .min(60, m.initial_setup_general_config_error_mfa_timeout_min()),
        public_proxy_url: z
          .url(m.initial_setup_general_config_error_public_proxy_url_invalid())
          .min(1, m.initial_setup_general_config_error_public_proxy_url_required()),
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
      Snackbar.error(m.initial_setup_general_config_error_save_failed());
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
        public_proxy_url: value.public_proxy_url,
      });
      mutate({
        defguard_url: value.defguard_url,
        default_admin_group_name: value.default_admin_group_name,
        default_authentication: value.default_authentication,
        default_mfa_code_lifetime: value.default_mfa_code_lifetime,
        public_proxy_url: value.public_proxy_url,
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
            {(field) => (
              <field.FormInput
                required
                label={m.initial_setup_general_config_label_defguard_url()}
                type="text"
              />
            )}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl} />
          <form.AppField name="default_admin_group_name">
            {(field) => (
              <field.FormInput
                required
                label={m.initial_setup_general_config_label_admin_group()}
                type="text"
              />
            )}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl} />
          <form.AppField name="default_authentication">
            {(field) => (
              <field.FormInput
                required
                label={m.initial_setup_general_config_label_auth_period()}
                type="number"
              />
            )}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl} />
          <form.AppField name="default_mfa_code_lifetime">
            {(field) => (
              <field.FormInput
                required
                label={m.initial_setup_general_config_label_mfa_timeout()}
                type="number"
              />
            )}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl} />
          <form.AppField name="public_proxy_url">
            {(field) => (
              <field.FormInput
                required
                label={m.initial_setup_general_config_label_public_proxy_url()}
                type="text"
              />
            )}
          </form.AppField>
        </form.AppForm>
      </form>
      <ModalControls
        submitProps={{
          text: m.initial_setup_controls_continue(),
          onClick: handleNext,
          loading: isPending,
        }}
      />
    </WizardCard>
  );
};
