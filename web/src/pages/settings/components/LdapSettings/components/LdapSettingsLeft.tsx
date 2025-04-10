import { Control } from 'react-hook-form';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { FormCheckBox } from '../../../../../shared/defguard-ui/components/Form/FormCheckBox/FormCheckBox';
import { FormInput } from '../../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { Helper } from '../../../../../shared/defguard-ui/components/Layout/Helper/Helper';
import { useAppStore } from '../../../../../shared/hooks/store/useAppStore';
import { SettingsLDAP } from '../../../../../shared/types';

type FormFields = Omit<SettingsLDAP, 'ldap_user_auxiliary_obj_classes'> & {
  ldap_user_auxiliary_obj_classes: string;
};

export const LdapSettingsLeft = ({ control }: { control: Control<FormFields> }) => {
  const { LL } = useI18nContext();
  const localLL = LL.settingsPage.ldapSettings;
  const enterpriseEnabled = useAppStore((s) => s.appInfo?.license_info.enterprise);

  return (
    <div className="left">
      <div>
        <div className="subsection-header helper-row">
          <h3>{localLL.form.headings.connection_settings()}</h3>
          <Helper>{localLL.form.helpers.connection_settings()}</Helper>
        </div>
        <div className="checkbox-column">
          <FormCheckBox
            controller={{ control, name: 'ldap_enabled' }}
            label={localLL.form.labels.ldap_enable()}
            labelPlacement="right"
            disabled={!enterpriseEnabled}
          />
          <FormCheckBox
            controller={{ control, name: 'ldap_use_starttls' }}
            label={localLL.form.labels.ldap_use_starttls()}
            labelPlacement="right"
            disabled={!enterpriseEnabled}
          />
          <FormCheckBox
            controller={{ control, name: 'ldap_uses_ad' }}
            label={localLL.form.labels.ldap_uses_ad()}
            labelPlacement="right"
            disabled={!enterpriseEnabled}
          />
          <FormCheckBox
            controller={{ control, name: 'ldap_tls_verify_cert' }}
            label={localLL.form.labels.ldap_tls_verify_cert()}
            labelPlacement="right"
            disabled={!enterpriseEnabled}
          />
        </div>
        <FormInput
          controller={{ control, name: 'ldap_url' }}
          label={localLL.form.labels.ldap_url()}
          disabled={!enterpriseEnabled}
        />
        <FormInput
          controller={{ control, name: 'ldap_bind_username' }}
          label={localLL.form.labels.ldap_bind_username()}
          disabled={!enterpriseEnabled}
        />
        <FormInput
          controller={{ control, name: 'ldap_bind_password' }}
          label={localLL.form.labels.ldap_bind_password()}
          type="password"
          disabled={!enterpriseEnabled}
        />
      </div>
      <div>
        <div className="subsection-header helper-row">
          <h3>{localLL.form.headings.user_settings()}</h3>
          <Helper>{localLL.form.helpers.user_settings()}</Helper>
        </div>
        <FormInput
          controller={{ control, name: 'ldap_username_attr' }}
          label={localLL.form.labels.ldap_username_attr()}
          disabled={!enterpriseEnabled}
        />
        <FormInput
          controller={{ control, name: 'ldap_user_search_base' }}
          label={localLL.form.labels.ldap_user_search_base()}
          disabled={!enterpriseEnabled}
        />
        <FormInput
          controller={{ control, name: 'ldap_user_obj_class' }}
          label={localLL.form.labels.ldap_user_obj_class()}
          disabled={!enterpriseEnabled}
          labelExtras={<Helper>{localLL.form.helpers.ldap_user_obj_class()}</Helper>}
        />
        <FormInput
          controller={{ control, name: 'ldap_user_auxiliary_obj_classes' }}
          label={localLL.form.labels.ldap_user_auxiliary_obj_classes()}
          disabled={!enterpriseEnabled}
          labelExtras={
            <Helper>{localLL.form.helpers.ldap_user_auxiliary_obj_classes()}</Helper>
          }
        />
        <FormInput
          controller={{ control, name: 'ldap_member_attr' }}
          label={localLL.form.labels.ldap_member_attr()}
          disabled={!enterpriseEnabled}
        />
      </div>
    </div>
  );
};
