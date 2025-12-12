import { useMemo } from 'react';
import type z from 'zod';
import { EvenSplit } from '../../../../../shared/defguard-ui/components/EvenSplit/EvenSplit';
import { SizedBox } from '../../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../../../shared/defguard-ui/types';
import { useAppForm } from '../../../../../shared/form';
import { formChangeLogic } from '../../../../../shared/formLogic';
import {
  directorySyncBehaviorOptions,
  directorySyncTargetOptions,
} from '../../../consts';
import { useAddExternalOpenIdStore } from '../../../useAddExternalOpenIdStore';
import { ProviderFormControls } from '../ProviderFormControls';
import { ProviderSyncToggle } from '../ProviderSyncToggle';
import { oktaProviderSyncSchema } from './schemas';
import type { ProviderFormProps } from './types';

type FormFields = z.infer<typeof oktaProviderSyncSchema>;

export const OktaProviderForm = ({ onSubmit }: ProviderFormProps) => {
  const providerState = useAddExternalOpenIdStore((s) => s.providerState);
  const back = useAddExternalOpenIdStore((s) => s.back);

  const defaultValues = useMemo(
    (): FormFields => ({
      directory_sync_admin_behavior: providerState.directory_sync_admin_behavior,
      directory_sync_interval: providerState.directory_sync_interval,
      directory_sync_target: providerState.directory_sync_target,
      directory_sync_user_behavior: providerState.directory_sync_user_behavior,
      okta_dirsync_client_id: providerState.okta_dirsync_client_id ?? '',
      okta_private_jwk: providerState.okta_private_jwk ?? '',
    }),
    [providerState],
  );

  const form = useAppForm({
    defaultValues,
    validationLogic: formChangeLogic,
    validators: {
      onSubmit: oktaProviderSyncSchema,
      onChange: oktaProviderSyncSchema,
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
        <ProviderSyncToggle>
          <EvenSplit>
            <form.AppField name="directory_sync_target">
              {(field) => (
                <field.FormSelect
                  options={directorySyncTargetOptions}
                  required
                  label="Synchronize"
                />
              )}
            </form.AppField>
            <form.AppField name="directory_sync_interval">
              {(field) => (
                <field.FormInput
                  type="number"
                  required
                  label="Synchronization interval"
                />
              )}
            </form.AppField>
          </EvenSplit>
          <SizedBox height={ThemeSpacing.Xl} />
          <EvenSplit>
            <form.AppField name="directory_sync_user_behavior">
              {(field) => (
                <field.FormSelect
                  required
                  label="User behavior"
                  options={directorySyncBehaviorOptions}
                />
              )}
            </form.AppField>
            <form.AppField name="directory_sync_admin_behavior">
              {(field) => (
                <field.FormSelect
                  required
                  label="Admin behavior"
                  options={directorySyncBehaviorOptions}
                />
              )}
            </form.AppField>
          </EvenSplit>
          <SizedBox height={ThemeSpacing.Xl} />
          <EvenSplit>
            <form.AppField name="okta_dirsync_client_id">
              {(field) => <field.FormInput required label="Directory sync client ID" />}
            </form.AppField>
            <form.AppField name="okta_private_jwk">
              {(field) => (
                <field.FormInput
                  required
                  label="Directory sync client private key"
                  type="password"
                />
              )}
            </form.AppField>
          </EvenSplit>
        </ProviderSyncToggle>
        <ProviderFormControls
          onBack={() => {
            back(form.state.values);
          }}
        />
      </form.AppForm>
    </form>
  );
};
