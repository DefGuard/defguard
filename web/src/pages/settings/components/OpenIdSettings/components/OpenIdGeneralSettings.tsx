import './style.scss';

import parse from 'html-react-parser';
import { UseFormReturn } from 'react-hook-form';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { FormCheckBox } from '../../../../../shared/defguard-ui/components/Form/FormCheckBox/FormCheckBox';
import { Helper } from '../../../../../shared/defguard-ui/components/Layout/Helper/Helper';
import { useAppStore } from '../../../../../shared/hooks/store/useAppStore';
import { OpenIdProvider } from '../../../../../shared/types';
import { useSettingsPage } from '../../../hooks/useSettingsPage';

export const OpenIdGeneralSettings = ({
  formControl,
}: {
  formControl: UseFormReturn<OpenIdProvider>;
}) => {
  const { LL } = useI18nContext();
  const localLL = LL.settingsPage.openIdSettings;

  const settings = useSettingsPage((state) => state.settings);
  const enterpriseEnabled = useAppStore((s) => s.appInfo?.license_info.enterprise);
  if (!settings) return null;

  return (
    <section id="openid-settings">
      <header>
        <h2>{localLL.general.title()}</h2>
        <Helper>{parse(localLL.general.helper())}</Helper>
      </header>
      <div>
        <div>
          <div className="checkbox-row">
            <FormCheckBox
              disabled={!enterpriseEnabled}
              label={localLL.general.createAccount.label()}
              controller={{
                control: formControl.control,
                name: 'create_account',
              }}
              labelPlacement="right"
            />
            <Helper>{localLL.general.createAccount.helper()}</Helper>
          </div>
        </div>
      </div>
    </section>
  );
};
