import './style.scss';

import parse from 'html-react-parser';
import { useMemo } from 'react';
import { useFormContext, useWatch } from 'react-hook-form';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { FormSelect } from '../../../../../shared/defguard-ui/components/Form/FormSelect/FormSelect';
import { Helper } from '../../../../../shared/defguard-ui/components/Layout/Helper/Helper';
import { LabeledCheckbox } from '../../../../../shared/defguard-ui/components/Layout/LabeledCheckbox/LabeledCheckbox';
import {
  SelectOption,
  SelectSizeVariant,
} from '../../../../../shared/defguard-ui/components/Layout/Select/types';
import { UsernameHandling } from './OpenIdSettingsForm';

export const OpenIdGeneralSettings = ({ isLoading }: { isLoading: boolean }) => {
  const { LL } = useI18nContext();
  const localLL = LL.settingsPage.openIdSettings;
  const { control, setValue } = useFormContext();
  const create_account = useWatch({
    control,
    name: 'create_account',
  }) as boolean;
  const use_openid_for_mfa = useWatch({
    control,
    name: 'use_openid_for_mfa',
  }) as boolean;
  const providerName = useWatch({
    control,
    name: 'name',
  }) as string;

  const options: SelectOption<UsernameHandling>[] = useMemo(
    () => [
      {
        value: 'RemoveForbidden',
        label: localLL.general.usernameHandling.options.remove(),
        key: 0,
      },
      {
        value: 'ReplaceForbidden',
        label: localLL.general.usernameHandling.options.replace(),
        key: 1,
      },
      {
        value: 'PruneEmailDomain',
        label: localLL.general.usernameHandling.options.prune_email(),
        key: 2,
      },
    ],
    [localLL.general.usernameHandling.options],
  );

  const providerConfigured = useMemo(() => {
    return providerName !== '';
  }, [providerName]);

  return (
    <div id="general-settings">
      <div className="subsection-header helper-row">
        <h3>{localLL.general.title()}</h3>
        <Helper>{parse(localLL.general.helper())}</Helper>
      </div>
      <div className="helper-row checkbox-padding">
        {/* FIXME: Really buggy when using the controller, investigate why */}
        <LabeledCheckbox
          label={localLL.general.createAccount.label()}
          // controller={{
          //   control,
          //   name: 'create_account',
          // }}
          value={create_account}
          onChange={(e) => {
            setValue('create_account', e);
          }}
          disabled={isLoading}
        />
        <Helper>{localLL.general.createAccount.helper()}</Helper>
      </div>
      <div className="helper-row checkbox-padding">
        {/* FIXME: Really buggy when using the controller, investigate why */}
        <LabeledCheckbox
          label={localLL.general.useOpenIdForMfa.label()}
          // controller={{
          //   control,
          //   name: 'use_openid_for_mfa',
          // }}
          value={providerConfigured ? use_openid_for_mfa : false}
          onChange={(e) => {
            setValue('use_openid_for_mfa', e);
          }}
          disabled={isLoading || !providerConfigured}
        />
        <Helper>{localLL.general.useOpenIdForMfa.helper()}</Helper>
      </div>
      <FormSelect
        controller={{
          control,
          name: 'username_handling',
        }}
        sizeVariant={SelectSizeVariant.STANDARD}
        options={options}
        label={localLL.general.usernameHandling.label()}
        labelExtras={<Helper>{localLL.general.usernameHandling.helper()}</Helper>}
        disabled={isLoading}
      />
    </div>
  );
};
