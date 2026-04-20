import { useMutation } from '@tanstack/react-query';
import { useMemo } from 'react';
import type z from 'zod';
import { useShallow } from 'zustand/react/shallow';
import { m } from '../../../../../paraglide/messages';
import { ProviderUsersSyncWarning } from '../../../../../shared/components/ProviderUsersSyncWarning/ProviderUsersSyncWarning';
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
import { jumpcloudProviderSyncSchema } from './schemas';
import type { ProviderFormProps } from './types';

type FormFields = z.infer<typeof jumpcloudProviderSyncSchema>;

export const JumpcloudProviderForm = ({ onSubmit }: ProviderFormProps) => {
  const providerState = useAddExternalOpenIdStore((s) => s.providerState);
  const [back] = useAddExternalOpenIdStore(useShallow((s) => [s.back]));

  const { mutate, isPending } = useMutation({
    mutationFn: onSubmit,
  });

  const defaultValues = useMemo(
    (): FormFields => ({
      directory_sync_admin_behavior: providerState.directory_sync_admin_behavior,
      directory_sync_interval: providerState.directory_sync_interval,
      directory_sync_target: providerState.directory_sync_target,
      directory_sync_user_behavior: providerState.directory_sync_user_behavior,
      jumpcloud_api_key: providerState.jumpcloud_api_key ?? '',
    }),
    [providerState],
  );

  const form = useAppForm({
    defaultValues,
    validationLogic: formChangeLogic,
    validators: {
      onSubmit: jumpcloudProviderSyncSchema,
      onChange: jumpcloudProviderSyncSchema,
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
          <ProviderUsersSyncWarning provider="JumpCloud" />
          <SizedBox height={ThemeSpacing.Xl} />
          <EvenSplit>
            <form.AppField name="directory_sync_target">
              {(field) => (
                <field.FormSelect
                  options={directorySyncTargetOptions}
                  required
                  label={m.settings_openid_provider_label_sync_target()}
                  helper={m.settings_openid_provider_helper_sync_target()}
                />
              )}
            </form.AppField>
            <form.AppField name="directory_sync_interval">
              {(field) => (
                <field.FormInput
                  type="number"
                  required
                  label={m.settings_openid_provider_label_sync_interval()}
                  helper={m.settings_openid_provider_helper_sync_interval()}
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
                  label={m.settings_openid_provider_label_sync_user_behavior()}
                  options={directorySyncBehaviorOptions}
                  helper={m.settings_openid_provider_helper_sync_user_behavior()}
                />
              )}
            </form.AppField>
            <form.AppField name="directory_sync_admin_behavior">
              {(field) => (
                <field.FormSelect
                  required
                  label={m.settings_openid_provider_label_sync_admin_behavior()}
                  options={directorySyncBehaviorOptions}
                  helper={m.settings_openid_provider_helper_sync_admin_behavior()}
                />
              )}
            </form.AppField>
          </EvenSplit>
          <SizedBox height={ThemeSpacing.Xl} />
          <form.AppField name="jumpcloud_api_key">
            {(field) => (
              <field.FormInput
                required
                label={m.settings_openid_provider_label_jumpcloud_api_key()}
                type="password"
                helper={m.settings_openid_provider_helper_jumpcloud_api_key()}
              />
            )}
          </form.AppField>
        </ProviderSyncToggle>
        <ProviderFormControls
          loading={isPending}
          onBack={() => {
            back(form.state.values);
          }}
          onNext={() => {
            mutate(form.state.values);
          }}
        />
      </form.AppForm>
    </form>
  );
};
