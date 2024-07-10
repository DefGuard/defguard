// import { zodResolver } from '@hookform/resolvers/zod';
// import { useMemo, useRef } from 'react';
// import { SubmitHandler, useForm } from 'react-hook-form';
// import { z } from 'zod';

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import IconCheckmarkWhite from '../../../../../shared/components/svg/IconCheckmarkWhite';
// import { FormInput } from '../../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { Button } from '../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../shared/defguard-ui/components/Layout/Button/types';
import useApi from '../../../../../shared/hooks/useApi';
// import { useToaster } from '../../../../../shared/hooks/useToaster';
import { QueryKeys } from '../../../../../shared/queries';
// import { useSettingsPage } from '../../../hooks/useSettingsPage';
import { ProviderDetails } from './ProviderDetails';
import { FormInput } from '../../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { zodResolver } from '@hookform/resolvers/zod';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useSettingsPage } from '../../../hooks/useSettingsPage';
import { SubmitHandler, useForm } from 'react-hook-form';
import { useToaster } from '../../../../../shared/hooks/useToaster';
import { OpenIdProvider, SettingsOpenID } from '../../../../../shared/types';
import { z } from 'zod';
import { Select } from '../../../../../shared/defguard-ui/components/Layout/Select/Select';
import {
  SelectSelectedValue,
  SelectSizeVariant,
} from '../../../../../shared/defguard-ui/components/Layout/Select/types';

type FormFields = SettingsOpenID;

export const OpenIdSettingsForm = () => {
  const { LL } = useI18nContext();
  const localLL = LL.settingsPage.openIdSettings;
  const submitRef = useRef<HTMLInputElement | null>(null);
  // const settings = useSettingsPage((state) => state.settings);
  // const setSettings = useSettingsPage((state) => state.setState);
  const [currentProvider, setCurrentProvider] = useState<OpenIdProvider | null>(null);

  const {
    settings: { patchSettings },
  } = useApi();

  const queryClient = useQueryClient();

  const {
    settings: {
      fetchOpenIdProviders,
      addOpenIdProvider,
      deleteOpenIdProvider,
      editOpenIdProvider,
    },
  } = useApi();
  const { data: providers, isLoading } = useQuery({
    queryFn: fetchOpenIdProviders,
    queryKey: [QueryKeys.FETCH_OPENID_PROVIDERS],
    refetchOnMount: true,
    refetchOnWindowFocus: false,
  });

  const toaster = useToaster();

  const { mutate } = useMutation({
    mutationFn: currentProvider ? editOpenIdProvider : addOpenIdProvider,
    onSuccess: () => {
      queryClient.invalidateQueries([QueryKeys.FETCH_OPENID_PROVIDERS]);
      toaster.success(LL.settingsPage.messages.editSuccess());
    },
    // TODO(aleksander): HANDLE ERROR
    onError: (error) => {
      toaster.error(error.message);
    },
  });

  useEffect(() => {
    if (providers && providers.length > 0) {
      setCurrentProvider(providers[0]);
    }
  }, [providers]);

  const schema = useMemo(
    () =>
      z.object({
        name: z.string().min(1, LL.form.error.required()),
        provider_url: z
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
      name: currentProvider?.name ?? '',
      provider_url: currentProvider?.provider_url ?? '',
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

  useEffect(() => {
    reset(defaultValues);
  }, [defaultValues]);

  console.log(currentProvider);

  const handleValidSubmit: SubmitHandler<FormFields> = (data) => {
    mutate(data);
  };

  const options = useMemo(
    () => [
      {
        key: 1,
        value: 'https://accounts.google.com',
        label: 'Google',
      },
      {
        key: 2,
        value: 'https://accounts.google2.com',
        label: 'Microsoft',
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

  const handleChange = async (val: string) => {
    if (!isLoading && currentProvider) {
      const newProvider: OpenIdProvider = {
        id: currentProvider.id,
        name: options.find((o) => o.value === val)?.label ?? '',
        provider_url: val,
        client_id: currentProvider.client_id,
        client_secret: currentProvider.client_secret,
      };
      setCurrentProvider(newProvider);
    }
  };

  return (
    <section id="openid-settings">
      <header>
        <h2>{localLL.title()}</h2>
      </header>
      <form id="openid-settings-form" onSubmit={handleSubmit(handleValidSubmit)}>
        {/* TODO(aleksander): Make a select here */}
        <FormInput
          controller={{ control, name: 'name' }}
          label={localLL.form.labels.name()}
        />
        <FormInput
          controller={{ control, name: 'provider_url' }}
          label={localLL.form.labels.provider_url()}
        />
        {/* <Select
        sizeVariant={SelectSizeVariant.SMALL}
        selected={settings?.enrollment_vpn_step_optional}
        options={vpnOptionalityOptions}
        renderSelected={renderSelectedVpn}
        onChangeSingle={(res) => handleChange(res)}
        loading={isLoading || isUndefined(settings)}
      /> */}
        <Select
          sizeVariant={SelectSizeVariant.SMALL}
          selected={currentProvider?.provider_url}
          options={options}
          renderSelected={renderSelected}
          onChangeSingle={(res) => handleChange(res)}
          loading={isLoading}
        />
        <FormInput
          controller={{ control, name: 'client_id' }}
          label={localLL.form.labels.client_id()}
        />
        <FormInput
          controller={{ control, name: 'client_secret' }}
          label={localLL.form.labels.client_secret()}
          type="password"
        />
        <input type="submit" aria-hidden="true" className="hidden" ref={submitRef} />
        <Button
          size={ButtonSize.SMALL}
          styleVariant={ButtonStyleVariant.SAVE}
          text={LL.common.controls.saveChanges()}
          type="submit"
          loading={isLoading}
          icon={<IconCheckmarkWhite />}
          onClick={() => submitRef.current?.click()}
        />
      </form>
    </section>
  );
};
