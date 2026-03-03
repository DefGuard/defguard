import { useMemo } from 'react';
import z from 'zod';
import { useShallow } from 'zustand/react/shallow';
import { m } from '../../../paraglide/messages';
import { Controls } from '../../../shared/components/Controls/Controls';
import { WizardCard } from '../../../shared/components/wizard/WizardCard/WizardCard';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import { useAppForm } from '../../../shared/form';
import { formChangeLogic } from '../../../shared/formLogic';
import { useMigrationWizardStore } from '../store/useMigrationWizardStore';

type FormFields = StoreValues;

type StoreValues = {
  defguard_url: string;
  default_admin_group_name: string;
  default_authentication: number;
  default_mfa_code_lifetime: number;
  public_proxy_url: string;
};

export const MigrationWizardGeneralConfigurationStep = () => {
  const defaultValues = useMigrationWizardStore(
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
          .url(m.migration_wizard_general_config_error_invalid_url())
          .min(1, m.migration_wizard_general_config_error_defguard_url_required()),
        default_admin_group_name: z
          .string()
          .min(1, m.migration_wizard_general_config_error_admin_group_required()),
        default_authentication: z
          .number()
          .min(1, m.migration_wizard_general_config_error_auth_period_min()),
        default_mfa_code_lifetime: z
          .number()
          .min(60, m.migration_wizard_general_config_error_mfa_timeout_min()),
        public_proxy_url: z
          .url(m.migration_wizard_general_config_error_public_proxy_url_invalid())
          .min(1, m.migration_wizard_general_config_error_public_proxy_url_required()),
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
      useMigrationWizardStore.setState({
        defguard_url: value.defguard_url,
        default_admin_group_name: value.default_admin_group_name,
        default_authentication_period_days: value.default_authentication,
        default_mfa_code_timeout_seconds: value.default_mfa_code_lifetime,
        public_proxy_url: value.public_proxy_url,
      });
      useMigrationWizardStore.getState().next();
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
          <form.AppField name="defguard_url">
            {(field) => (
              <field.FormInput
                required
                label={m.migration_wizard_general_config_label_defguard_url()}
                type="text"
              />
            )}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl} />
          <form.AppField name="default_admin_group_name">
            {(field) => (
              <field.FormInput
                required
                label={m.migration_wizard_general_config_label_admin_group()}
                type="text"
              />
            )}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl} />
          <form.AppField name="default_authentication">
            {(field) => (
              <field.FormInput
                required
                label={m.migration_wizard_general_config_label_auth_period()}
                type="number"
              />
            )}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl} />
          <form.AppField name="default_mfa_code_lifetime">
            {(field) => (
              <field.FormInput
                required
                label={m.migration_wizard_general_config_label_mfa_timeout()}
                type="number"
              />
            )}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl} />
          <form.AppField name="public_proxy_url">
            {(field) => (
              <field.FormInput
                required
                label={m.migration_wizard_general_config_label_public_proxy_url()}
                type="text"
              />
            )}
          </form.AppField>
          <Controls>
            <Button
              variant="outlined"
              text={m.controls_back()}
              onClick={() => {
                useMigrationWizardStore.getState().back();
              }}
            />
            <div className="right">
              <Button text={m.controls_continue()} type="submit" />
            </div>
          </Controls>
        </form.AppForm>
      </form>
    </WizardCard>
  );
};
