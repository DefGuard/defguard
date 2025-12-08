import { useQuery } from '@tanstack/react-query';
import { Link } from '@tanstack/react-router';
import type { Settings } from '../../../shared/api/types';
import { Breadcrumbs } from '../../../shared/components/Breadcrumbs/Breadcrumbs';
import { Page } from '../../../shared/components/Page/Page';
import { SettingsCard } from '../../../shared/components/SettingsCard/SettingsCard';
import { SettingsHeader } from '../../../shared/components/SettingsHeader/SettingsHeader';
import { SettingsLayout } from '../../../shared/components/SettingsLayout/SettingsLayout';
import { isPresent } from '../../../shared/defguard-ui/utils/isPresent';
import { getSettingsQueryOptions } from '../../../shared/query';

const breadcrumbs = [
  <Link to="/settings" search={{ tab: 'general' }} key={0}>
    General
  </Link>,
  <Link to="/settings/client" key={1}>
    Client behavior
  </Link>,
];

export const SettingsClientPage = () => {
  const { data: settings } = useQuery(getSettingsQueryOptions);
  return (
    <Page title="Settings">
      <Breadcrumbs links={breadcrumbs} />
      <SettingsLayout>
        <SettingsHeader
          icon="user"
          title="Client behavior"
          subtitle="Manage user permissions and configuration options for device control, WireGuard setup, and VPN routing."
        />
        {isPresent(settings) && (
          <SettingsCard>
            <Content settings={settings} />
          </SettingsCard>
        )}
      </SettingsLayout>
    </Page>
  );
};

const Content = ({ settings }: { settings: Settings }) => {
  return <p>{JSON.stringify(settings)}</p>;
};
