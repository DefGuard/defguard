import './style.scss';

import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import parse from 'html-react-parser';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { z } from 'zod';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import IconCheckmarkWhite from '../../../../../shared/components/svg/IconCheckmarkWhite';
import { FormCheckBox } from '../../../../../shared/defguard-ui/components/Form/FormCheckBox/FormCheckBox';
import { FormInput } from '../../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { FormSelect } from '../../../../../shared/defguard-ui/components/Form/FormSelect/FormSelect';
import { Button } from '../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../shared/defguard-ui/components/Layout/Button/types';
import { Helper } from '../../../../../shared/defguard-ui/components/Layout/Helper/Helper';
import { Select } from '../../../../../shared/defguard-ui/components/Layout/Select/Select';
import {
  SelectOption,
  SelectSelectedValue,
  SelectSizeVariant,
} from '../../../../../shared/defguard-ui/components/Layout/Select/types';
import SvgIconDownload from '../../../../../shared/defguard-ui/components/svg/IconDownload';
import { useAppStore } from '../../../../../shared/hooks/store/useAppStore';
import useApi from '../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../shared/hooks/useToaster';
import { QueryKeys } from '../../../../../shared/queries';
import { OpenIdProvider } from '../../../../../shared/types';
import { titleCase } from '../../../../../shared/utils/titleCase';

type FormFields = OpenIdProvider;

const SUPPORTED_SYNC_PROVIDERS = ['Google'];

export const OpenIdSettingsForm = () => {
  const { LL } = useI18nContext();
  const localLL = LL.settingsPage.openIdSettings;
  const [currentProvider, setCurrentProvider] = useState<OpenIdProvider | null>(null);
  const queryClient = useQueryClient();
  const docsLink =
    // eslint-disable-next-line max-len
    'https://docs.defguard.net/enterprise/all-enteprise-features/external-openid-providers';
  const enterpriseEnabled = useAppStore((state) => state.enterprise_status?.enabled);
  const [googleServiceAccountFileName, setGoogleServiceAccountFileName] = useState<
    string | null
  >(null);

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
    }),
    [currentProvider],
  );

  const { handleSubmit, reset, control, setValue } = useForm<FormFields>({
    resolver: zodResolver(schema),
    defaultValues,
    mode: 'all',
  });

  // Make sure the form data is fresh
  useEffect(() => {
    reset(defaultValues);
  }, [defaultValues, reset]);

  const handleValidSubmit: SubmitHandler<FormFields> = (data) => {
    mutate(data);
  };

  const handleDeleteProvider = useCallback(() => {
    if (currentProvider) {
      deleteProvider(currentProvider.name);
      setCurrentProvider(null);
    }
  }, [currentProvider, deleteProvider]);

  const options: SelectOption<string>[] = useMemo(
    () => [
      {
        value: 'Google',
        label: 'Google',
        key: 1,
      },
      {
        value: 'Microsoft',
        label: 'Microsoft',
        key: 2,
      },
      {
        value: 'Custom',
        label: localLL.form.custom(),
        key: 3,
      },
    ],
    [localLL.form],
  );

  const renderSelected = useCallback(
    (selected: string): SelectSelectedValue => {
      const option = options.find((o) => o.value === selected);

      if (!option) throw Error("Selected value doesn't exist");

      return {
        key: option.key,
        displayValue: option.label,
      };
    },
    [options],
  );

  const getProviderUrl = useCallback(({ name }: { name: string }): string | null => {
    switch (name) {
      case 'Google':
        return 'https://accounts.google.com';
      case 'Microsoft':
        return `https://login.microsoftonline.com/<TENANT_ID>/v2.0`;
      default:
        return null;
    }
  }, []);

  const getProviderDisplayName = useCallback(
    ({ name }: { name: string }): string | null => {
      switch (name) {
        case 'Google':
          return 'Google';
        case 'Microsoft':
          return 'Microsoft';
        default:
          return null;
      }
    },
    [],
  );

  const handleChange = useCallback(
    (val: string) => {
      setCurrentProvider({
        ...currentProvider,
        id: currentProvider?.id ?? 0,
        name: val,
        base_url: getProviderUrl({ name: val }) ?? '',
        client_id: currentProvider?.client_id ?? '',
        client_secret: currentProvider?.client_secret ?? '',
        display_name:
          getProviderDisplayName({ name: val }) ?? currentProvider?.display_name ?? '',
        google_service_account_email: currentProvider?.google_service_account_email ?? '',
        google_service_account_key: currentProvider?.google_service_account_key ?? '',
        admin_email: currentProvider?.admin_email ?? '',
        directory_sync_enabled: currentProvider?.directory_sync_enabled ?? false,
        directory_sync_interval: currentProvider?.directory_sync_interval ?? 600,
        directory_sync_user_behavior:
          currentProvider?.directory_sync_user_behavior ?? 'keep',
        directory_sync_admin_behavior:
          currentProvider?.directory_sync_admin_behavior ?? 'keep',
      });
    },
    [currentProvider, getProviderUrl, getProviderDisplayName],
  );

  const userBehaviorOptions = useMemo(
    () => [
      {
        value: 'keep',
        label: 'Keep',
        key: 1,
      },
      {
        value: 'disable',
        label: 'Disable',
        key: 2,
      },
      {
        value: 'delete',
        label: 'Delete',
        key: 3,
      },
    ],
    [],
  );

  return (
    <section id="openid-settings">
      <header>
        <h2>{localLL.form.title()}</h2>

        <Helper>{parse(localLL.form.helper())}</Helper>
        <div className="controls">
          <Button
            size={ButtonSize.SMALL}
            styleVariant={ButtonStyleVariant.SAVE}
            text={LL.common.controls.saveChanges()}
            type="submit"
            loading={isLoading}
            form="openid-settings-form"
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
      </header>
      <form id="openid-settings-form" onSubmit={handleSubmit(handleValidSubmit)}>
        <Select
          sizeVariant={SelectSizeVariant.STANDARD}
          selected={currentProvider?.name ?? undefined}
          options={options}
          renderSelected={renderSelected}
          onChangeSingle={(res) => handleChange(res)}
          label={localLL.form.labels.provider.label()}
          labelExtras={<Helper>{parse(localLL.form.labels.provider.helper())}</Helper>}
          disabled={!enterpriseEnabled}
        />
        <FormInput
          controller={{ control, name: 'base_url' }}
          label={localLL.form.labels.base_url.label()}
          labelExtras={<Helper>{parse(localLL.form.labels.base_url.helper())}</Helper>}
          disabled={currentProvider?.name === 'Google' || !enterpriseEnabled}
          required
        />
        <FormInput
          controller={{ control, name: 'client_id' }}
          label={localLL.form.labels.client_id.label()}
          labelExtras={<Helper>{parse(localLL.form.labels.client_id.helper())}</Helper>}
          disabled={!enterpriseEnabled}
          required
        />
        <FormInput
          controller={{ control, name: 'client_secret' }}
          label={localLL.form.labels.client_secret.label()}
          labelExtras={
            <Helper>{parse(localLL.form.labels.client_secret.helper())}</Helper>
          }
          required
          type="password"
          disabled={!enterpriseEnabled}
        />
        <FormInput
          controller={{ control, name: 'display_name' }}
          label={localLL.form.labels.display_name.label()}
          labelExtras={
            <Helper>{parse(localLL.form.labels.display_name.helper())}</Helper>
          }
          disabled={!enterpriseEnabled || currentProvider?.name !== 'Custom'}
        />
        <header id="dirsync-header">
          <h3>{localLL.form.directory_sync_settings.title()}</h3>
          <Helper>{localLL.form.directory_sync_settings.helper()}</Helper>
        </header>
        <div id="directory-sync-settings">
          {SUPPORTED_SYNC_PROVIDERS.includes(currentProvider?.name ?? '') ? (
            currentProvider?.name === 'Google' ? (
              <>
                <div id="enable-dir-sync">
                  <FormCheckBox
                    disabled={isLoading || !enterpriseEnabled}
                    label={localLL.form.labels.enable_directory_sync.label()}
                    labelPlacement="right"
                    controller={{ control, name: 'directory_sync_enabled' }}
                  />
                </div>
                <FormInput
                  value={currentProvider?.directory_sync_interval ?? ''}
                  controller={{ control, name: 'directory_sync_interval' }}
                  type="number"
                  name="directory_sync_interval"
                  label={localLL.form.labels.sync_interval.label()}
                  required
                  labelExtras={
                    <Helper>{parse(localLL.form.labels.sync_interval.helper())}</Helper>
                  }
                />
                <FormSelect
                  controller={{ control, name: 'directory_sync_user_behavior' }}
                  options={userBehaviorOptions}
                  label={localLL.form.labels.user_behavior.label()}
                  renderSelected={(val) => ({
                    key: val,
                    displayValue: titleCase(val),
                  })}
                  labelExtras={
                    <Helper>{parse(localLL.form.labels.user_behavior.helper())}</Helper>
                  }
                />
                <FormSelect
                  controller={{ control, name: 'directory_sync_admin_behavior' }}
                  options={userBehaviorOptions}
                  label={localLL.form.labels.admin_behavior.label()}
                  renderSelected={(val) => ({
                    key: val,
                    displayValue: titleCase(val),
                  })}
                  labelExtras={
                    <Helper>{parse(localLL.form.labels.admin_behavior.helper())}</Helper>
                  }
                />
                <FormInput
                  controller={{ control, name: 'admin_email' }}
                  label={localLL.form.labels.admin_email.label()}
                  disabled={!enterpriseEnabled}
                  labelExtras={
                    <Helper>{parse(localLL.form.labels.admin_email.helper())}</Helper>
                  }
                />
                <div className="hidden-input">
                  <FormInput
                    value={currentProvider?.google_service_account_key ?? ''}
                    type="text"
                    name="google_service_account_key"
                    controller={{ control, name: 'google_service_account_key' }}
                    readOnly
                  />
                </div>
                <FormInput
                  value={currentProvider?.google_service_account_email ?? ''}
                  controller={{ control, name: 'google_service_account_email' }}
                  type="text"
                  name="google_service_account_email"
                  readOnly
                  label={localLL.form.labels.service_account_used.label()}
                  labelExtras={
                    <Helper>
                      {parse(localLL.form.labels.service_account_used.helper())}
                    </Helper>
                  }
                />
                <div className="input">
                  <div className="top">
                    <label className="input-label">
                      {localLL.form.labels.service_account_key_file.label()}:
                    </label>
                    <Helper>
                      {localLL.form.labels.service_account_key_file.helper()}
                    </Helper>
                  </div>
                  <div className="file-upload-container">
                    <input
                      className="file-upload"
                      type="file"
                      accept=".json"
                      onChange={(e) => {
                        const file = e.target.files?.[0];
                        if (file) {
                          const reader = new FileReader();
                          reader.onload = (e) => {
                            const key = JSON.parse(e.target?.result as string);
                            setValue('google_service_account_key', key.private_key);
                            setValue('google_service_account_email', key.client_email);
                            setGoogleServiceAccountFileName(file.name);
                          };
                          reader.readAsText(file);
                        }
                      }}
                    />
                    <div className="upload-label">
                      <SvgIconDownload />{' '}
                      <p>
                        {googleServiceAccountFileName
                          ? `${localLL.form.labels.service_account_key_file.uploaded()}: ${googleServiceAccountFileName}`
                          : localLL.form.labels.service_account_key_file.uploadPrompt()}
                      </p>
                    </div>
                  </div>
                </div>
              </>
            ) : null
          ) : (
            <p id="sync-not-supported">
              {localLL.form.directory_sync_settings.notSupported()}
            </p>
          )}
        </div>
      </form>
      <a href={docsLink} target="_blank" rel="noreferrer">
        {localLL.form.documentation()}
      </a>
    </section>
  );
};
