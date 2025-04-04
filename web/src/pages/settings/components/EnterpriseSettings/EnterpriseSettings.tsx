import parse from 'html-react-parser';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { BigInfoBox } from '../../../../shared/defguard-ui/components/Layout/BigInfoBox/BigInfoBox';
import { useAppStore } from '../../../../shared/hooks/store/useAppStore';
import { useSettingsPage } from '../../hooks/useSettingsPage';
import { EnterpriseForm } from './components/EnterpriseForm';

export const EnterpriseSettings = () => {
  const enterpriseInfo = useSettingsPage((s) => s.enterpriseInfo);
  const { LL } = useI18nContext();
  const localLL = LL.settingsPage.enterpriseOnly;
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
      <div className="column-layout">
        {!appInfo.license_info.enterprise && (
          <div className="enterprise-info-backdrop">
            <div className="enterprise-info">
              <div>
                <h2>{localLL.title()}</h2>
                {enterpriseInfo?.expired && <p>{localLL.currentExpired()}</p>}
                <p>
                  {localLL.subtitle()}{' '}
                  <a
                    href="https://defguard.net/pricing/"
                    target="_blank"
                    rel="noreferrer"
                  >
                    {localLL.website()}
                  </a>
                  .
                </p>
              </div>
            </div>
          </div>
        )}
        <div className="left">
          <EnterpriseForm />
        </div>
        <div className="right"></div>
      </div>
    </>
  );
};
