import './style.scss';

import { useQuery } from '@tanstack/react-query';
import parse from 'html-react-parser';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { BigInfoBox } from '../../../../shared/defguard-ui/components/Layout/BigInfoBox/BigInfoBox';
import { LoaderSpinner } from '../../../../shared/defguard-ui/components/Layout/LoaderSpinner/LoaderSpinner';
import useApi from '../../../../shared/hooks/useApi';
import { QueryKeys } from '../../../../shared/queries';
import { OpenIdGeneralSettings } from './components/OpenIdGeneralSettings';
import { OpenIdSettingsForm } from './components/OpenIdSettingsForm';

export const OpenIdSettings = () => {
  const { LL } = useI18nContext();
  const localLL = LL.settingsPage.enterpriseOnly;
  const { getEnterpriseInfo } = useApi();
  const { data: enterpriseInfo, isLoading } = useQuery({
    queryFn: getEnterpriseInfo,
    queryKey: [QueryKeys.FETCH_ENTERPRISE_INFO],
    refetchOnMount: true,
    refetchOnWindowFocus: false,
  });
  if (isLoading) {
    return (
      <div className="spinner-container">
        <LoaderSpinner size={100} />
      </div>
    );
  }

  return (
    <>
      {!enterpriseInfo?.enabled && (
        <div className="enterprise-info-backdrop">
          <div className="enterprise-info">
            <div>
              <h2>{localLL.title()}</h2>
              {/* If enterprise is disabled but we have some license info, we may assume that the license has expired */}
              {enterpriseInfo?.license_info && <p>{localLL.currentExpired()}</p>}
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
      {!enterpriseInfo?.needs_license && !enterpriseInfo?.license_info && (
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
