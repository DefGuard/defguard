import './style.scss';

import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
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
import { Select } from '../../../../../shared/defguard-ui/components/Layout/Select/Select';
import {
  SelectOption,
  SelectSelectedValue,
  SelectSizeVariant,
} from '../../../../../shared/defguard-ui/components/Layout/Select/types';
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

  const {
    settings: { fetchOpenIdProviders, addOpenIdProvider },
  } = useApi();
  const { data: provider, isLoading } = useQuery({
    queryFn: fetchOpenIdProviders,
    queryKey: [QueryKeys.FETCH_OPENID_PROVIDERS],
    refetchOnMount: true,
    refetchOnWindowFocus: false,
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

  useEffect(() => {
    if (provider) {
      setCurrentProvider(provider);
    }
  }, [provider]);

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
    }),
    [currentProvider],
  );

  const { handleSubmit, reset, control } = useForm<FormFields>({
    resolver: zodResolver(schema),
    defaultValues,
    mode: 'all',
  });

  // Make sure the form is refresh
  useEffect(() => {
    reset(defaultValues);
  }, [defaultValues, reset]);

  const handleValidSubmit: SubmitHandler<FormFields> = (data) => {
    mutate(data);
  };

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
        label: 'Custom',
        key: 3,
      },
    ],
    [],
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

  const handleChange = async (val: string) => {
    console.log(currentProvider?.base_url);
    setCurrentProvider({
      id: currentProvider?.id ?? 0,
      name: val,
      base_url: getProviderUrl({ name: val }) ?? '',
      client_id: currentProvider?.client_id ?? '',
      client_secret: currentProvider?.client_secret ?? '',
    });
  };

  return (
    <section id="openid-settings">
      <header>
        <h2>{localLL.title()}</h2>
        <Button
          size={ButtonSize.SMALL}
          styleVariant={ButtonStyleVariant.SAVE}
          text={LL.common.controls.saveChanges()}
          type="submit"
          loading={isLoading}
          form="openid-settings-form"
          icon={<IconCheckmarkWhite />}
        />
      </header>
      <form id="openid-settings-form" onSubmit={handleSubmit(handleValidSubmit)}>
        <FormInput
          controller={{ control, name: 'client_id' }}
          label={localLL.form.labels.client_id()}
        />
        <FormInput
          controller={{ control, name: 'client_secret' }}
          label={localLL.form.labels.client_secret()}
          type="password"
        />
        <Select
          sizeVariant={SelectSizeVariant.STANDARD}
          selected={currentProvider?.name ?? undefined}
          options={options}
          renderSelected={renderSelected}
          onChangeSingle={(res) => handleChange(res)}
          label={localLL.form.labels.provider()}
        />
        {currentProvider?.name !== 'Google' && (
          <FormInput
            controller={{ control, name: 'base_url' }}
            label={localLL.form.labels.base_url()}
          />
        )}
      </form>
    </section>
  );
};
