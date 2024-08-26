import { useI18nContext } from '../../../../i18n/i18n-react';
import { useAppStore } from '../../../../shared/hooks/store/useAppStore';
import { EnterpriseForm } from './components/EnterpriseForm';

export const EnterpriseSettings = () => {
  const enterpriseEnabled = useAppStore((state) => state.enterprise_enabled);
  const { LL } = useI18nContext();
  const localLL = LL.settingsPage.enterpriseOnly;
  return (
    <>
      {!enterpriseEnabled && (
        <div className="enterprise-info-backdrop">
          <div className="enterprise-info">
            <div>
              <h2>{localLL.title()}</h2>
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