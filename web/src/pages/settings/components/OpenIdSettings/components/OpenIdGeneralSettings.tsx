import './style.scss';

import parse from 'html-react-parser';
import { useFormContext, useWatch } from 'react-hook-form';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { Helper } from '../../../../../shared/defguard-ui/components/Layout/Helper/Helper';
import { LabeledCheckbox } from '../../../../../shared/defguard-ui/components/Layout/LabeledCheckbox/LabeledCheckbox';
import { FormSelect } from '../../../../../shared/defguard-ui/components/Form/FormSelect/FormSelect';
import {
  SelectOption,
  SelectSizeVariant,
} from '../../../../../shared/defguard-ui/components/Layout/Select/types';
import { UsernameHandling } from './OpenIdSettingsForm';
import { useMemo } from 'react';

export const OpenIdGeneralSettings = ({ isLoading }: { isLoading: boolean }) => {
  const { LL } = useI18nContext();
  const localLL = LL.settingsPage.openIdSettings;
  const { control, setValue } = useFormContext();
  const create_account = useWatch({
    control,
    name: 'create_account',
  }) as boolean;

  const options: SelectOption<UsernameHandling>[] = useMemo(
    () => [
      {
        value: 'RemoveForbidden',
        label: 'Remove forbidden characters',
        key: 0,
      },
      {
        value: 'ReplaceForbidden',
        label: 'Replace forbidden characters',
        key: 1,
      },
      {
        value: 'PruneEmailDomain',
        label: 'Prune email domain',
        key: 2,
      },
    ],
    [localLL.form],
  );
  return (
    <div id="general-settings">
      <div className="subsection-header helper-row">
        <h3>{localLL.general.title()}</h3>
        <Helper>{parse(localLL.general.helper())}</Helper>
      </div>
      <div className="helper-row">
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
