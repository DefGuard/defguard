import { useI18nContext } from '../../../../i18n/i18n-react';
import { useAppStore } from '../../../../shared/hooks/store/useAppStore';
import { EnterpriseForm } from './components/EnterpriseForm';

export const EnterpriseSettings = () => {
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
              {/* If enterprise is disabled but we have some license info, the license probably expired */}
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
      <div className="left">
        <EnterpriseForm />
      </div>
      <div className="right"></div>
    </>
  );
};
