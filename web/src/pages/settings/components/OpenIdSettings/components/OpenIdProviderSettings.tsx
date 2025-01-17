import './style.scss';

import parse from 'html-react-parser';
import { useCallback, useMemo } from 'react';
import { useFormContext, useWatch } from 'react-hook-form';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { FormInput } from '../../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { FormSelect } from '../../../../../shared/defguard-ui/components/Form/FormSelect/FormSelect';
import { Helper } from '../../../../../shared/defguard-ui/components/Layout/Helper/Helper';
import {
  SelectOption,
  SelectSelectedValue,
  SelectSizeVariant,
} from '../../../../../shared/defguard-ui/components/Layout/Select/types';

export const OpenIdSettingsForm = ({ isLoading }: { isLoading: boolean }) => {
  const { LL } = useI18nContext();
  const localLL = LL.settingsPage.openIdSettings;
  const { control, setValue } = useFormContext();

  const options: SelectOption<string>[] = useMemo(
    () => [
      {
        value: '',
        label: localLL.form.none(),
        key: 0,
      },
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

  const providerName = useWatch({
    control,
    name: 'name',
  }) as string;

  const handleProviderChange = useCallback(
    (val: string) => {
      setValue('base_url', getProviderUrl({ name: val }) ?? '');
      setValue('display_name', getProviderDisplayName({ name: val }) ?? '');
    },
    [getProviderUrl, getProviderDisplayName, setValue],
  );

  return (
    <section id="openid-settings">
      <header>
        <h2>{localLL.form.title()}</h2>
        <Helper>{parse(localLL.form.helper())}</Helper>
      </header>
      <FormSelect
        controller={{
          control,
          name: 'name',
        }}
        sizeVariant={SelectSizeVariant.STANDARD}
        options={options}
        renderSelected={renderSelected}
        onChangeSingle={(res) => handleProviderChange(res)}
        label={localLL.form.labels.provider.label()}
        labelExtras={<Helper>{parse(localLL.form.labels.provider.helper())}</Helper>}
        disabled={isLoading}
      />
      <FormInput
        controller={{ control, name: 'base_url' }}
        label={localLL.form.labels.base_url.label()}
        labelExtras={<Helper>{parse(localLL.form.labels.base_url.helper())}</Helper>}
        disabled={providerName === 'Google' || isLoading}
        required
      />
      <FormInput
        controller={{ control, name: 'client_id' }}
        label={localLL.form.labels.client_id.label()}
        labelExtras={<Helper>{parse(localLL.form.labels.client_id.helper())}</Helper>}
        disabled={isLoading}
        required
      />
      <FormInput
        controller={{ control, name: 'client_secret' }}
        label={localLL.form.labels.client_secret.label()}
        labelExtras={<Helper>{parse(localLL.form.labels.client_secret.helper())}</Helper>}
        required
        type="password"
        disabled={isLoading}
      />
      <FormInput
        controller={{ control, name: 'display_name' }}
        label={localLL.form.labels.display_name.label()}
        labelExtras={<Helper>{parse(localLL.form.labels.display_name.helper())}</Helper>}
        disabled={isLoading || providerName !== 'Custom'}
      />
      <a
        href={
          'https://docs.defguard.net/enterprise/all-enteprise-features/external-openid-providers'
        }
        target="_blank"
        rel="noreferrer"
      >
        {localLL.form.documentation()}
      </a>
    </section>
  );
};
