import './style.scss';

import parse from 'html-react-parser';
import { useI18nContext } from '../../../../i18n/i18n-react';
import { BigInfoBox } from '../../../../shared/defguard-ui/components/Layout/BigInfoBox/BigInfoBox';
import { useAppStore } from '../../../../shared/hooks/store/useAppStore';
import { OpenIdGeneralSettings } from './components/OpenIdGeneralSettings';
import { OpenIdSettingsForm } from './components/OpenIdSettingsForm';

export const OpenIdSettings = () => {
  const enterpriseStatus = useAppStore((state) => state.enterprise_status);
  const { LL } = useI18nContext();
  const localLL = LL.settingsPage.enterpriseOnly;

  return (
    <>
      {!enterpriseStatus?.enabled && (
        <div className="enterprise-info-backdrop">
          <div className="enterprise-info">
            <div>
              <h2>{localLL.title()}</h2>
              {/* If enterprise is disabled but we have some license info, we may assume that the license has expired */}
              {enterpriseStatus?.license_info && <p>{localLL.currentExpired()}</p>}
              <p>
                {localLL.subtitle()}{' '}
                <a href="https://defguard.net/pricing/" target="_blank" rel="noreferrer">
                  {localLL.website()}
                </a>
                .
              </p>
            </div>
          </div>
        </div>
      )}
      {!enterpriseStatus?.needs_license && !enterpriseStatus?.license_info && (
        <div className="license-not-required-container">
          <BigInfoBox
            message={parse(LL.settingsPage.license.licenseInfo.licenseNotRequired())}
          />
        </div>
      )}
      <div className="left">
        <OpenIdSettingsForm />
      </div>
      <div className="right">
        <OpenIdGeneralSettings />
      </div>
    </>
  );
};
