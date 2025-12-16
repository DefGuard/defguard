import { omit } from 'lodash-es';
import { useMemo } from 'react';
import z from 'zod';
import { m } from '../../../../paraglide/messages';
import { EditPageControls } from '../../../../shared/components/EditPageControls/EditPageControls';
import { EditPageFormSection } from '../../../../shared/components/EditPageFormSection/EditPageFormSection';
import { Fold } from '../../../../shared/defguard-ui/components/Fold/Fold';
import { SizedBox } from '../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../../shared/defguard-ui/types';
import { useAppForm } from '../../../../shared/form';
import { formChangeLogic } from '../../../../shared/formLogic';
import {
  directorySyncBehaviorOptions,
  directorySyncTargetOptions,
  formatMicrosoftBaseUrl,
  providerUsernameHandlingOptions,
} from '../../../AddExternalOpenIdWizardPage/consts';
import {
  baseExternalProviderConfigSchema,
  microsoftProviderSyncSchema,
} from '../../../AddExternalOpenIdWizardPage/steps/AddExternalOpenIdDirectoryStep/forms/schemas';
import type { EditProviderFormProps } from '../types';

const basicSchema = z
  .object({
    directory_sync_enabled: z.boolean(),
    microsoftTenantId: z
      .string(m.form_error_required())
      .trim()
      .min(1, m.form_error_required()),
  })
  .extend(omit(baseExternalProviderConfigSchema.shape, ['base_url']));

const syncSchema = basicSchema.extend(microsoftProviderSyncSchema.shape);

const discriminatedSchema = z.discriminatedUnion('directory_sync_enabled', [
  basicSchema,
  syncSchema,
]);

type FormFields = z.infer<typeof discriminatedSchema>;

export const EditMicrosoftProviderForm = ({
  provider,
  loading,
  onDelete,
  onSubmit,
}: EditProviderFormProps) => {
  const defaultValues = useMemo((): FormFields => {
    const tenantId = provider.base_url.split('/')[provider.base_url.length - 2];
    return {
      client_id: provider.client_id,
      client_secret: provider.client_secret,
      create_account: provider.create_account,
      display_name: provider.display_name,
      username_handling: provider.username_handling,
      directory_sync_admin_behavior: provider.directory_sync_admin_behavior,
      directory_sync_interval: provider.directory_sync_interval,
      directory_sync_target: provider.directory_sync_target,
      directory_sync_user_behavior: provider.directory_sync_user_behavior,
      directory_sync_enabled: provider.directory_sync_enabled,
      directory_sync_group_match: provider.directory_sync_group_match ?? '',
      microsoftTenantId: tenantId,
    };
  }, [provider]);

  const form = useAppForm({
    defaultValues,
    validationLogic: formChangeLogic,
    validators: {
      onSubmit: syncSchema,
      onChange: syncSchema,
    },
    onSubmit: async ({ value }) => {
      const base_url = formatMicrosoftBaseUrl(value.microsoftTenantId);
      await onSubmit({ ...value, base_url });
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
        <EditPageFormSection label="Client settings">
          <form.AppField name="display_name">
            {(field) => <field.FormInput required label="Display name" />}
          </form.AppField>
          <form.AppField name="microsoftTenantId">
            {(field) => <field.FormInput required label="Tenant ID" />}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl2} />
          <form.AppField name="client_id">
            {(field) => <field.FormInput required label="Client ID" />}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl2} />
          <form.AppField name="client_secret">
            {(field) => (
              <field.FormInput type="password" required label="Client secret" />
            )}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl2} />
          <form.AppField name="username_handling">
            {(field) => (
              <field.FormSelect
                options={providerUsernameHandlingOptions}
                label="Username handling"
              />
            )}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl2} />
          <form.AppField name="create_account">
            {(field) => (
              <field.FormInteractiveBlock
                variant="checkbox"
                title={`Automatically create user account when logging in for the first time through external OpenID.`}
                content={`If this option is enabled, Defguard automatically creates new accounts for users who log in for the first time using an external OpenID. Otherwise, the user account must first be created by an administrator.`}
              />
            )}
          </form.AppField>
        </EditPageFormSection>
        <EditPageFormSection label="Directory synchronization">
          <form.AppField name="directory_sync_enabled">
            {(field) => <field.FormToggle label="Directory synchronization" />}
          </form.AppField>
          <form.Subscribe selector={(s) => s.values.directory_sync_enabled}>
            {(enabled) => (
              <Fold open={enabled}>
                <SizedBox height={ThemeSpacing.Xl3} />
                <form.AppField name="directory_sync_target">
                  {(field) => (
                    <field.FormSelect
                      options={directorySyncTargetOptions}
                      label="Synchronize"
                    />
                  )}
                </form.AppField>
                <SizedBox height={ThemeSpacing.Xl2} />
                <form.AppField name="directory_sync_interval">
                  {(field) => (
                    <field.FormInput
                      required
                      label="Synchronize interval"
                      type="number"
                    />
                  )}
                </form.AppField>
                <SizedBox height={ThemeSpacing.Xl2} />
                <form.AppField name="directory_sync_user_behavior">
                  {(field) => (
                    <field.FormSelect
                      options={directorySyncBehaviorOptions}
                      label="User behavior"
                    />
                  )}
                </form.AppField>
                <SizedBox height={ThemeSpacing.Xl2} />
                <form.AppField name="directory_sync_admin_behavior">
                  {(field) => (
                    <field.FormSelect
                      options={directorySyncBehaviorOptions}
                      label="Admin behavior"
                    />
                  )}
                </form.AppField>
                <SizedBox height={ThemeSpacing.Xl2} />
                <form.AppField name="directory_sync_group_match">
                  {(field) => <field.FormInput label="Sync only matching groups" />}
                </form.AppField>
                <SizedBox height={ThemeSpacing.Xl2} />
                <form.AppField name="prefetch_users">
                  {(field) => <field.FormCheckbox text="Prefetch users" />}
                </form.AppField>
              </Fold>
            )}
          </form.Subscribe>
        </EditPageFormSection>
        <form.Subscribe selector={(s) => s.isSubmitting}>
          {(submitting) => (
            <EditPageControls
              deleteProps={{
                disabled: submitting,
                text: 'Delete provider',
                onClick: onDelete,
                loading: loading,
              }}
              cancelProps={{
                text: m.controls_cancel(),
                disabled: submitting || loading,
                onClick: () => {
                  window.history.back();
                },
              }}
              submitProps={{
                text: m.controls_save_changes(),
                loading: submitting || loading,
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
