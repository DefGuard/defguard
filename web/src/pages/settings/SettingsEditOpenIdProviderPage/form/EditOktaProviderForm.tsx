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
  providerUsernameHandlingOptions,
} from '../../../AddExternalOpenIdWizardPage/consts';
import {
  baseExternalProviderConfigSchema,
  oktaProviderSyncSchema,
} from '../../../AddExternalOpenIdWizardPage/steps/AddExternalOpenIdDirectoryStep/forms/schemas';
import type { EditProviderFormProps } from '../types';

const basicSchema = z
  .object({
    directory_sync_enabled: z.boolean(),
  })
  .extend(baseExternalProviderConfigSchema.shape);

const syncSchema = basicSchema.extend(oktaProviderSyncSchema.shape);

const discriminatedSchema = z.discriminatedUnion('directory_sync_enabled', [
  basicSchema,
  syncSchema,
]);

type FormFields = z.infer<typeof discriminatedSchema>;

export const EditOktaProviderForm = ({
  provider,
  loading,
  onDelete,
  onSubmit,
}: EditProviderFormProps) => {
  const defaultValues = useMemo((): FormFields => {
    return {
      base_url: provider.base_url,
      okta_dirsync_client_id: provider.okta_dirsync_client_id ?? '',
      okta_private_jwk: provider.okta_private_jwk ?? '',
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
        <EditPageFormSection label="Client settings">
          <form.AppField name="display_name">
            {(field) => <field.FormInput required label="Display name" />}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl2} />
          <form.AppField name="base_url">
            {(field) => <field.FormInput required label="Base URL" />}
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
                <form.AppField name="okta_dirsync_client_id">
                  {(field) => (
                    <field.FormInput required label="Directory sync client ID" />
                  )}
                </form.AppField>
                <SizedBox height={ThemeSpacing.Xl2} />
                <form.AppField name="okta_private_jwk">
                  {(field) => (
                    <field.FormInput
                      required
                      label="Directory sync client private key"
                      type="password"
                    />
                  )}
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
