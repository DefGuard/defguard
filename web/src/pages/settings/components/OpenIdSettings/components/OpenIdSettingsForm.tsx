import './style.scss';

import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import parse from 'html-react-parser';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { z } from 'zod';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import IconCheckmarkWhite from '../../../../../shared/components/svg/IconCheckmarkWhite';
import { FormInput } from '../../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
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
import { useAppStore } from '../../../../../shared/hooks/store/useAppStore';
import useApi from '../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../shared/hooks/useToaster';
import { QueryKeys } from '../../../../../shared/queries';
import { OpenIdProvider } from '../../../../../shared/types';

type FormFields = OpenIdProvider;

export const OpenIdSettingsForm = () => {
  const { LL } = useI18nContext();
  const localLL = LL.settingsPage.openIdSettings;
  const [currentProvider, setCurrentProvider] = useState<OpenIdProvider | null>(null);
  const queryClient = useQueryClient();
  const docsLink =
    'https://defguard.gitbook.io/defguard/admin-and-features/external-openid-providers';
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
    }),
    [currentProvider],
  );

  const { handleSubmit, reset, control } = useForm<FormFields>({
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
        id: currentProvider?.id ?? 0,
        name: val,
        base_url: getProviderUrl({ name: val }) ?? '',
        client_id: currentProvider?.client_id ?? '',
        client_secret: currentProvider?.client_secret ?? '',
        display_name:
          getProviderDisplayName({ name: val }) ?? currentProvider?.display_name ?? '',
      });
    },
    [currentProvider, getProviderUrl, getProviderDisplayName],
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
        />
        <FormInput
          controller={{ control, name: 'client_id' }}
          label={localLL.form.labels.client_id.label()}
          labelExtras={<Helper>{parse(localLL.form.labels.client_id.helper())}</Helper>}
          disabled={!enterpriseEnabled}
        />
        <FormInput
          controller={{ control, name: 'client_secret' }}
          label={localLL.form.labels.client_secret.label()}
          labelExtras={
            <Helper>{parse(localLL.form.labels.client_secret.helper())}</Helper>
          }
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
      </form>
      <a href={docsLink} target="_blank" rel="noreferrer">
        {localLL.form.documentation()}
      </a>
    </section>
  );
};
