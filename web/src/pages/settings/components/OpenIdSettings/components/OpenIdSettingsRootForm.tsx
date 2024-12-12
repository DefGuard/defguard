import './style.scss';

import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { z } from 'zod';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import IconCheckmarkWhite from '../../../../../shared/components/svg/IconCheckmarkWhite';
import { Button } from '../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../shared/defguard-ui/components/Layout/Button/types';
import { LoaderSpinner } from '../../../../../shared/defguard-ui/components/Layout/LoaderSpinner/LoaderSpinner';
import { useAppStore } from '../../../../../shared/hooks/store/useAppStore';
import useApi from '../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../shared/hooks/useToaster';
import { QueryKeys } from '../../../../../shared/queries';
import { OpenIdProvider } from '../../../../../shared/types';
import { DirsyncSettings } from './DirectorySyncSettings';
import { OpenIdGeneralSettings } from './OpenIdGeneralSettings';
import { OpenIdSettingsForm } from './OpenIdProviderSettings';

type FormFields = OpenIdProvider;

export const OpenIdSettingsRootForm = () => {
  const { LL } = useI18nContext();
  const localLL = LL.settingsPage.openIdSettings;
  const [currentProvider, setCurrentProvider] = useState<OpenIdProvider | null>(null);
  const queryClient = useQueryClient();
  const enterpriseEnabled = useAppStore((state) => state.enterprise_status?.enabled);

  const {
    settings: { fetchOpenIdProviders, addOpenIdProvider, deleteOpenIdProvider },
  } = useApi();

  const { isLoading } = useQuery({
    queryFn: fetchOpenIdProviders,
    queryKey: [QueryKeys.FETCH_OPENID_PROVIDERS],
    refetchOnMount: true,
    refetchOnWindowFocus: false,
    onSuccess: (provider) => {
      setCurrentProvider(provider);
    },
    retry: false,
    enabled: enterpriseEnabled,
  });

  const toaster = useToaster();

  const { mutate } = useMutation({
    mutationFn: addOpenIdProvider,
    onSuccess: () => {
      queryClient.invalidateQueries([QueryKeys.FETCH_OPENID_PROVIDERS]);
      toaster.success(LL.settingsPage.messages.editSuccess());
    },
    onError: (error) => {
      toaster.error(LL.messages.error());
      console.error(error);
    },
  });

  const { mutate: deleteProvider } = useMutation({
    mutationFn: deleteOpenIdProvider,
    onSuccess: () => {
      queryClient.invalidateQueries([QueryKeys.FETCH_OPENID_PROVIDERS]);
      toaster.success(LL.settingsPage.messages.editSuccess());
    },
    onError: (error) => {
      toaster.error(LL.messages.error());
      console.error(error);
    },
  });

  const schema = useMemo(
    () =>
      z.object({
        name: z.string().min(1, LL.form.error.required()),
        base_url: z
          .string()
          .url(LL.form.error.invalid())
          .min(1, LL.form.error.required()),
        client_id: z.string().min(1, LL.form.error.required()),
        client_secret: z.string().min(1, LL.form.error.required()),
        display_name: z.string(),
        admin_email: z.string(),
        google_service_account_email: z.string(),
        google_service_account_key: z.string(),
        directory_sync_enabled: z.boolean(),
        directory_sync_interval: z.number().min(60, LL.form.error.invalid()),
        directory_sync_user_behavior: z.string(),
        directory_sync_admin_behavior: z.string(),
        directory_sync_target: z.string(),
      }),
    [LL.form.error],
  );

  const defaultValues = useMemo(
    (): FormFields => ({
      id: currentProvider?.id ?? 0,
      name: currentProvider?.name ?? '',
      base_url: currentProvider?.base_url ?? '',
      client_id: currentProvider?.client_id ?? '',
      client_secret: currentProvider?.client_secret ?? '',
      display_name: currentProvider?.display_name ?? '',
      admin_email: currentProvider?.admin_email ?? '',
      google_service_account_email: currentProvider?.google_service_account_email ?? '',
      google_service_account_key: currentProvider?.google_service_account_key ?? '',
      directory_sync_enabled: currentProvider?.directory_sync_enabled ?? false,
      directory_sync_interval: currentProvider?.directory_sync_interval ?? 600,
      directory_sync_user_behavior:
        currentProvider?.directory_sync_user_behavior ?? 'keep',
      directory_sync_admin_behavior:
        currentProvider?.directory_sync_admin_behavior ?? 'keep',
      directory_sync_target: currentProvider?.directory_sync_target ?? 'all',
    }),
    [currentProvider],
  );

  const formControl = useForm<FormFields>({
    resolver: zodResolver(schema),
    defaultValues,
    mode: 'all',
  });

  const { handleSubmit, reset } = formControl;

  // Make sure the form data is fresh
  useEffect(() => {
    reset(defaultValues);
  }, [defaultValues, reset]);

  const conditionallyRequired: (keyof OpenIdProvider)[] = [
    'admin_email',
    'google_service_account_email',
  ];

  const handleValidSubmit: SubmitHandler<FormFields> = (data) => {
    // Some fields are required only if directory sync is enabled.
    // Check if the required fields are filled in.
    const formValues = formControl.getValues();
    const dirsync_enabled = formValues.directory_sync_enabled;
    if (dirsync_enabled) {
      const missingRequiredFields = conditionallyRequired.filter(
        (field) =>
          formValues[field]?.toString().length === 0 || formValues[field] === null,
      );
      if (missingRequiredFields.length) {
        for (const field of missingRequiredFields) {
          formControl.setError(field, {
            type: 'required',
            message: LL.form.error.required(),
          });
        }
        return;
      }
    }
    mutate(data);
  };

  const handleDeleteProvider = useCallback(() => {
    if (currentProvider) {
      deleteProvider(currentProvider.name);
      setCurrentProvider(null);
    }
  }, [currentProvider, deleteProvider]);

  return (
    <form id="root-form" onSubmit={handleSubmit(handleValidSubmit)}>
      <div className="controls">
        <Button
          size={ButtonSize.SMALL}
          styleVariant={ButtonStyleVariant.SAVE}
          text={LL.common.controls.saveChanges()}
          type="submit"
          loading={isLoading}
          form="root-form"
          icon={<IconCheckmarkWhite />}
          disabled={!enterpriseEnabled}
        />
        <Button
          text={localLL.form.delete()}
          size={ButtonSize.SMALL}
          styleVariant={ButtonStyleVariant.CONFIRM}
          loading={isLoading}
          onClick={() => {
            handleDeleteProvider();
          }}
          disabled={!enterpriseEnabled}
        />
      </div>
      {isLoading ? (
        <div className="loader">
          <LoaderSpinner size={80} />
        </div>
      ) : (
        <>
          <div className="left">
            <OpenIdSettingsForm
              currentProvider={currentProvider}
              setCurrentProvider={setCurrentProvider}
              formControl={formControl}
            />
          </div>
          <div className="right">
            <OpenIdGeneralSettings />
            <DirsyncSettings
              currentProvider={currentProvider}
              formControl={formControl}
            />
          </div>
        </>
      )}
    </form>
  );
};
