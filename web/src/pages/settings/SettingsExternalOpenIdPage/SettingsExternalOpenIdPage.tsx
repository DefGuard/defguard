import { useNavigate } from '@tanstack/react-router';
import { SettingsHeader } from '../../../shared/components/SettingsHeader/SettingsHeader';
import { SettingsLayout } from '../../../shared/components/SettingsLayout/SettingsLayout';
import { ExternalProviderCard } from './components/ExternalProviderCard/ExternalProviderCard';
import './style.scss';
import { useQuery } from '@tanstack/react-query';
import { useCallback, useMemo } from 'react';
import api from '../../../shared/api/api';
import {
  OpenIdProviderKind,
  type OpenIdProviderKindValue,
} from '../../../shared/api/types';
import { businessBadgeProps } from '../../../shared/components/badges/BusinessBadge';
import { InfoBanner } from '../../../shared/defguard-ui/components/InfoBanner/InfoBanner';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import { isPresent } from '../../../shared/defguard-ui/utils/isPresent';
import { getLicenseInfoQueryOptions } from '../../../shared/query';
import { canUseBusinessFeature, licenseActionCheck } from '../../../shared/utils/license';
import { useAddExternalOpenIdStore } from '../../AddExternalOpenIdWizardPage/useAddExternalOpenIdStore';

export const SettingsExternalOpenIdPage = () => {
  const navigate = useNavigate();

  const { data: activeProvider } = useQuery({
    queryFn: api.openIdProvider.getOpenIdProvider,
    queryKey: ['openid', 'provider'],
    select: (resp) => resp.data?.provider,
  });

  const { data: licenseInfo, isFetching: licenseLoading } = useQuery(
    getLicenseInfoQueryOptions,
  );

  const visibleProviders = useMemo(() => {
    const res = Object.values(OpenIdProviderKind).filter(
      (p) => p !== OpenIdProviderKind.Zitadel,
    );
    if (activeProvider) {
      return res.filter((p) => p !== activeProvider.name);
    }
    return res;
  }, [activeProvider]);

  const handleAddProvider = useCallback(
    (provider: OpenIdProviderKindValue) => {
      if (licenseInfo === undefined) return;

      licenseActionCheck(canUseBusinessFeature(licenseInfo), () => {
        useAddExternalOpenIdStore.getState().initialize(provider);
        navigate({
          to: '/add-external-openid',
          replace: true,
        });
      });
    },
    [licenseInfo, navigate],
  );

  const handleEditProvider = useCallback(() => {
    if (licenseInfo === undefined || !isPresent(activeProvider)) return;

    licenseActionCheck(canUseBusinessFeature(licenseInfo), () => {
      navigate({ to: '/settings/edit-openid' });
    });
  }, [licenseInfo, activeProvider, navigate]);

  return (
    <SettingsLayout id="settings-openid-tab">
      <SettingsHeader
        icon="openid"
        title="External OpenID settings"
        badgeProps={businessBadgeProps}
        subtitle="Manage user permissions and configuration options for device control, WireGuard setup, and VPN routing."
      />
      {isPresent(activeProvider) && (
        <>
          <p className="section-label">{'Active external ID Providers'}</p>
          <SizedBox height={ThemeSpacing.Md} />
          <ExternalProviderCard
            edit
            provider={activeProvider.name as OpenIdProviderKindValue}
            displayName={activeProvider.display_name}
            loading={licenseLoading}
            onClick={handleEditProvider}
          />
          <SizedBox height={ThemeSpacing.Xl3} />
          <p className="section-label">{'Other external ID providers'}</p>
          <SizedBox height={ThemeSpacing.Md} />
          <InfoBanner
            variant="warning"
            icon="info-outlined"
            text={
              'We currently support only one external ID provider, but we plan to add support for multiple providers in the near future.'
            }
          />
          <SizedBox height={ThemeSpacing.Md} />
        </>
      )}
      <div className="providers">
        {visibleProviders.map((provider) => (
          <ExternalProviderCard
            loading={licenseLoading}
            disabled={isPresent(activeProvider)}
            provider={provider}
            key={provider}
            onClick={() => {
              handleAddProvider(provider);
            }}
          />
        ))}
      </div>
    </SettingsLayout>
  );
};
