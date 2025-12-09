import { Link } from '@tanstack/react-router';
import { Breadcrumbs } from '../../../shared/components/Breadcrumbs/Breadcrumbs';
import { Page } from '../../../shared/components/Page/Page';
import { SettingsHeader } from '../../../shared/components/SettingsHeader/SettingsHeader';
import { SettingsLayout } from '../../../shared/components/SettingsLayout/SettingsLayout';
import { higherPlanBadgeProps } from '../shared/consts';
import { ExternalProvider } from '../shared/types';
import { ExternalProviderCard } from './components/ExternalProviderCard/ExternalProviderCard';
import './style.scss';

const breadcrumbs = [
  <Link
    to="/settings"
    search={{
      tab: 'openid',
    }}
    key={0}
  >
    External identity providers
  </Link>,
  <Link to="/settings/openid" key={1}>
    External OpenID settings
  </Link>,
];

export const SettingsExternalOpenIdPage = () => {
  return (
    <Page title="Settings" id="settings-external-openid">
      <Breadcrumbs links={breadcrumbs} />
      <SettingsLayout>
        <SettingsHeader
          icon="openid"
          title="External OpenID settings"
          badgeProps={higherPlanBadgeProps}
          subtitle="Manage user permissions and configuration options for device control, WireGuard setup, and VPN routing."
        />
        <div className="providers">
          {Object.values(ExternalProvider).map((provider) => (
            <ExternalProviderCard provider={provider} key={provider} onClick={() => {}} />
          ))}
        </div>
      </SettingsLayout>
    </Page>
  );
};
