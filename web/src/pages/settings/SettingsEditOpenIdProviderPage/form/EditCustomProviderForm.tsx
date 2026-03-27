import { useMemo } from 'react';
import type z from 'zod';
import { m } from '../../../../paraglide/messages';
import { EditPageControls } from '../../../../shared/components/EditPageControls/EditPageControls';
import { EditPageFormSection } from '../../../../shared/components/EditPageFormSection/EditPageFormSection';
import { SizedBox } from '../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../../shared/defguard-ui/types';
import { useAppForm } from '../../../../shared/form';
import { formChangeLogic } from '../../../../shared/formLogic';
import { providerUsernameHandlingOptions } from '../../../AddExternalOpenIdWizardPage/consts';
import { baseExternalProviderConfigSchema } from '../../../AddExternalOpenIdWizardPage/steps/AddExternalOpenIdDirectoryStep/forms/schemas';
import type { EditProviderFormProps } from '../types';

const formSchema = baseExternalProviderConfigSchema;

type FormFields = z.infer<typeof formSchema>;

export const EditCustomProviderForm = ({
  provider,
  onDelete,
  onSubmit,
}: EditProviderFormProps) => {
  const defaultValues = useMemo((): FormFields => {
    return {
      client_id: provider.client_id,
      client_secret: provider.client_secret,
      create_account: provider.create_account,
      display_name: provider.display_name,
      username_handling: provider.username_handling,
      base_url: provider.base_url,
    };
  }, [provider]);

  const form = useAppForm({
    defaultValues,
    validationLogic: formChangeLogic,
    validators: {
      onSubmit: formSchema,
      onChange: formSchema,
    },
    onSubmit: async ({ value }) => {
      await onSubmit(value);
    },
  });

  return (
    <form
      onSubmit={(e) => {
        e.stopPropagation();
        e.preventDefault();
        form.handleSubmit();
      }}
    >
      <form.AppForm>
        <EditPageFormSection label={m.settings_openid_provider_client_settings_title()}>
          <form.AppField name="display_name">
            {(field) => (
              <field.FormInput
                required
                label="settings_openid_provider_label_display_name"
              />
            )}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl2} />
          <form.AppField name="base_url">
            {(field) => (
              <field.FormInput required label="settings_openid_provider_label_base_url" />
            )}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl2} />
          <form.AppField name="client_id">
            {(field) => (
              <field.FormInput
                required
                label="settings_openid_provider_label_client_id"
              />
            )}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl2} />
          <form.AppField name="client_secret">
            {(field) => (
              <field.FormInput
                type="password"
                required
                label="settings_openid_provider_label_client_secret"
              />
            )}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl2} />
          <form.AppField name="username_handling">
            {(field) => (
              <field.FormSelect
                options={providerUsernameHandlingOptions}
                label="settings_openid_provider_label_username_handling"
              />
            )}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl2} />
          <form.AppField name="create_account">
            {(field) => (
              <field.FormInteractiveBlock
                variant="checkbox"
                title={m.settings_openid_provider_create_account_title()}
                content={m.settings_openid_provider_create_account_content()}
              />
            )}
          </form.AppField>
        </EditPageFormSection>
        <form.Subscribe selector={(s) => s.isSubmitting}>
          {(submitting) => (
            <EditPageControls
              deleteProps={{
                disabled: submitting,
                text: m.settings_openid_provider_delete_button(),
                onClick: onDelete,
              }}
              cancelProps={{
                text: m.controls_cancel(),
                disabled: submitting,
                onClick: () => {
                  window.history.back();
                },
              }}
              submitProps={{
                text: m.controls_save_changes(),
                loading: submitting,
                type: 'submit',
                onClick: () => {
                  form.handleSubmit();
                },
              }}
            />
          )}
        </form.Subscribe>
      </form.AppForm>
    </form>
  );
};
