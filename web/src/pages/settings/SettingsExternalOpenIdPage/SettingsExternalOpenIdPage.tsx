import { useNavigate } from '@tanstack/react-router';
import { SettingsHeader } from '../../../shared/components/SettingsHeader/SettingsHeader';
import { SettingsLayout } from '../../../shared/components/SettingsLayout/SettingsLayout';
import { higherPlanBadgeProps } from '../shared/consts';
import { ExternalProviderCard } from './components/ExternalProviderCard/ExternalProviderCard';
import './style.scss';
import { useQuery } from '@tanstack/react-query';
import { useMemo } from 'react';
import api from '../../../shared/api/api';
import {
  OpenIdProviderKind,
  type OpenIdProviderKindValue,
} from '../../../shared/api/types';
import { InfoBanner } from '../../../shared/defguard-ui/components/InfoBanner/InfoBanner';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import { isPresent } from '../../../shared/defguard-ui/utils/isPresent';
import { useAddExternalOpenIdStore } from '../../AddExternalOpenIdWizardPage/useAddExternalOpenIdStore';

export const SettingsExternalOpenIdPage = () => {
  const navigate = useNavigate();

  const { data: activeProvider } = useQuery({
    queryFn: api.openIdProvider.getOpenIdProvider,
    queryKey: ['openid', 'provider'],
    select: (resp) => resp.data?.provider,
  });

  const visibleProviders = useMemo(() => {
    const res = Object.values(OpenIdProviderKind).filter(
      (p) => p !== OpenIdProviderKind.Zitadel,
    );
    if (activeProvider) {
      return res.filter((p) => p !== activeProvider.name);
    }
    return res;
  }, [activeProvider]);

  return (
    <SettingsLayout id="settings-openid-tab">
      <SettingsHeader
        icon="openid"
        title="External OpenID settings"
        badgeProps={higherPlanBadgeProps}
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
            onClick={() => {
              navigate({ to: '/settings/edit-openid' });
            }}
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
            disabled={isPresent(activeProvider)}
            provider={provider}
            key={provider}
            onClick={() => {
              useAddExternalOpenIdStore.getState().initialize(provider);
              navigate({
                to: '/add-external-openid',
                replace: true,
              });
            }}
          />
        ))}
      </div>
    </SettingsLayout>
  );
};
