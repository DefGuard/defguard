import './style.scss';

import parse from 'html-react-parser';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { BigInfoBox } from '../../../../shared/defguard-ui/components/Layout/BigInfoBox/BigInfoBox';
import { useAppStore } from '../../../../shared/hooks/store/useAppStore';
import { LdapSettingsForm } from './components/LdapSettingsForm';

export const LdapSettings = () => {
  const { LL } = useI18nContext();
  const appInfo = useAppStore((s) => s.appInfo);

  if (!appInfo) return null;
  return (
    <>
      {appInfo.license_info.is_enterprise_free && (
        <div className="license-not-required-container">
          <BigInfoBox
            message={parse(LL.settingsPage.license.licenseInfo.licenseNotRequired())}
          />
        </div>
      )}
      <LdapSettingsForm />
    </>
  );
};
