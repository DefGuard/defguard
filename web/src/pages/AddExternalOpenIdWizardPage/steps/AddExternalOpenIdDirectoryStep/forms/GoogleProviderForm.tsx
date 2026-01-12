import { useMutation } from '@tanstack/react-query';
import { omit } from 'lodash-es';
import { useCallback, useMemo } from 'react';
import type z from 'zod';
import { m } from '../../../../../paraglide/messages';
import { DescriptionBlock } from '../../../../../shared/components/DescriptionBlock/DescriptionBlock';
import { EvenSplit } from '../../../../../shared/defguard-ui/components/EvenSplit/EvenSplit';
import { SizedBox } from '../../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../../../shared/defguard-ui/types';
import { isPresent } from '../../../../../shared/defguard-ui/utils/isPresent';
import { useAppForm } from '../../../../../shared/form';
import { formChangeLogic } from '../../../../../shared/formLogic';
import {
  directorySyncBehaviorOptions,
  directorySyncTargetOptions,
} from '../../../consts';
import { useAddExternalOpenIdStore } from '../../../useAddExternalOpenIdStore';
import { ProviderFormControls } from '../ProviderFormControls';
import { ProviderSyncToggle } from '../ProviderSyncToggle';
import {
  googleProviderSyncSchema,
  parseGoogleKeyFile,
  providerToGoogleKeyFile,
} from './schemas';
import type { ProviderFormProps } from './types';

type FormFields = z.infer<typeof googleProviderSyncSchema>;

export const GoogleProviderForm = ({ onSubmit }: ProviderFormProps) => {
  const storeValues = useAddExternalOpenIdStore((s) => s.providerState);
  const back = useAddExternalOpenIdStore((s) => s.back);

  const defaultValues = useMemo(
    (): FormFields => ({
      admin_email: storeValues.admin_email ?? '',
      directory_sync_admin_behavior: storeValues.directory_sync_admin_behavior,
      directory_sync_interval: storeValues.directory_sync_interval,
      directory_sync_target: storeValues.directory_sync_target,
      directory_sync_user_behavior: storeValues.directory_sync_user_behavior,
      google_service_account_file: providerToGoogleKeyFile(
        storeValues.google_service_account_key,
        storeValues.google_service_account_email,
      ),
    }),
    [storeValues],
  );

  const form = useAppForm({
    defaultValues,
    validationLogic: formChangeLogic,
    validators: {
      onSubmit: googleProviderSyncSchema,
      onChange: googleProviderSyncSchema,
    },
    onSubmit: async ({ value, formApi }) => {
      const fileData = await parseGoogleKeyFile(
        value.google_service_account_file as File,
      );
      if (isPresent(fileData)) {
        await onSubmit({
          ...value,
          google_service_account_email: fileData?.client_email ?? '',
          google_service_account_key: fileData?.private_key ?? '',
        });
      } else {
        formApi.setErrorMap({
          onSubmit: {
            fields: {
              google_service_account_file:
                value.google_service_account_file === null
                  ? m.form_error_required()
                  : m.form_error_file_contents(),
            },
          },
        });
      }
    },
  });

  const toStore = useCallback(async (state: FormFields) => {
    const fileData = await parseGoogleKeyFile(state.google_service_account_file as File);
    return {
      ...omit(state, ['google_service_account_file']),
      google_service_account_key: fileData?.private_key ?? null,
      google_service_account_email: fileData?.client_email ?? null,
    };
  }, []);

  const { mutate, isPending } = useMutation({
    mutationFn: async () => {
      return onSubmit(await toStore(form.state.values));
    },
  });

  const handleBack = useCallback(async () => {
    back(await toStore(form.state.values));
  }, [form.state.values, back, toStore]);

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
          <form.AppField name="admin_email">
            {(field) => <field.FormInput required label="Admin email" />}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl3} />
          <DescriptionBlock title="Service account key">
            <p>{`Upload a new service account key file to set the service account used for synchronization. NOTE: The uploaded file won't be visible after saving the settings and reloading the page as it's contents are sensitive and are never sent back to the dashboard.`}</p>
          </DescriptionBlock>
          <SizedBox height={ThemeSpacing.Xl} />
          <form.AppField name="google_service_account_file">
            {(field) => <field.FormUploadField />}
          </form.AppField>
        </ProviderSyncToggle>
        <ProviderFormControls
          loading={isPending}
          onBack={handleBack}
          onNext={() => {
            mutate();
          }}
        />
      </form.AppForm>
    </form>
  );
};
