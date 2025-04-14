import { useMemo } from 'react';
import { Control } from 'react-hook-form';
import ReactMarkdown from 'react-markdown';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { FormCheckBox } from '../../../../../shared/defguard-ui/components/Form/FormCheckBox/FormCheckBox';
import { FormInput } from '../../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { FormSelect } from '../../../../../shared/defguard-ui/components/Form/FormSelect/FormSelect';
import { Helper } from '../../../../../shared/defguard-ui/components/Layout/Helper/Helper';
import { MessageBox } from '../../../../../shared/defguard-ui/components/Layout/MessageBox/MessageBox';
import { MessageBoxType } from '../../../../../shared/defguard-ui/components/Layout/MessageBox/types';
import {
  SelectOption,
  SelectSizeVariant,
} from '../../../../../shared/defguard-ui/components/Layout/Select/types';
import { useAppStore } from '../../../../../shared/hooks/store/useAppStore';
import { SettingsLDAP } from '../../../../../shared/types';

type FormFields = Omit<SettingsLDAP, 'ldap_user_auxiliary_obj_classes'> & {
  ldap_user_auxiliary_obj_classes: string;
};

export const LdapSettingsRight = ({ control }: { control: Control<FormFields> }) => {
  const { LL } = useI18nContext();
  const localLL = LL.settingsPage.ldapSettings;

  const enterpriseEnabled = useAppStore((s) => s.appInfo?.license_info.enterprise);

  const options: SelectOption<boolean>[] = useMemo(
    () => [
      {
        value: false,
        label: 'Defguard',
        key: 0,
      },
      {
        value: true,
        label: 'LDAP',
        key: 1,
      },
    ],
    [],
  );

  return (
    <div className="right">
      <div>
        <div className="helper-row subsection-header">
          <h3>{localLL.form.headings.group_settings()}</h3>
          <Helper>{localLL.form.helpers.group_settings()}</Helper>
        </div>
        <FormInput
          controller={{ control, name: 'ldap_groupname_attr' }}
          label={localLL.form.labels.ldap_groupname_attr()}
          disabled={!enterpriseEnabled}
        />
        <FormInput
          controller={{ control, name: 'ldap_group_obj_class' }}
          label={localLL.form.labels.ldap_group_obj_class()}
          disabled={!enterpriseEnabled}
          labelExtras={<Helper>{localLL.form.helpers.ldap_group_obj_class()}</Helper>}
        />
        <FormInput
          controller={{ control, name: 'ldap_group_member_attr' }}
          label={localLL.form.labels.ldap_group_member_attr()}
          disabled={!enterpriseEnabled}
        />
        <FormInput
          controller={{ control, name: 'ldap_group_search_base' }}
          label={localLL.form.labels.ldap_group_search_base()}
          disabled={!enterpriseEnabled}
        />
      </div>
      <div>
        <div className="helper-row subsection-header">
          <h3>{localLL.sync.header()}</h3>
          <Helper>{localLL.sync.helpers.heading()}</Helper>
        </div>
        <MessageBox type={MessageBoxType.INFO}>
          <ReactMarkdown>{localLL.sync.info()}</ReactMarkdown>
        </MessageBox>
        <div className="checkbox-column">
          <div className="helper-row">
            <FormCheckBox
              controller={{ control, name: 'ldap_sync_enabled' }}
              label={localLL.form.labels.ldap_sync_enabled()}
              labelPlacement="right"
              disabled={!enterpriseEnabled}
            />
            <Helper>{localLL.sync.helpers.sync_enabled()}</Helper>
          </div>
        </div>
        <FormSelect
          controller={{ control, name: 'ldap_is_authoritative' }}
          sizeVariant={SelectSizeVariant.STANDARD}
          options={options}
          label={localLL.form.labels.ldap_authoritative_source()}
          labelExtras={<Helper>{localLL.sync.helpers.authority()}</Helper>}
          disabled={!enterpriseEnabled}
        />
        <FormInput
          controller={{ control, name: 'ldap_sync_interval' }}
          label={localLL.form.labels.ldap_sync_interval()}
          type="number"
          disabled={!enterpriseEnabled}
          labelExtras={<Helper>{localLL.sync.helpers.interval()}</Helper>}
        />
        <FormInput
          controller={{ control, name: 'ldap_sync_groups' }}
          label={localLL.form.labels.ldap_sync_groups()}
          labelExtras={<Helper>{localLL.sync.helpers.groups()}</Helper>}
        />
      </div>
    </div>
  );
};
