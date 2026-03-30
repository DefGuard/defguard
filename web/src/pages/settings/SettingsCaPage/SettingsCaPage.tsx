import { Link } from '@tanstack/react-router';
import { m } from '../../../paraglide/messages';
import type { Settings } from '../../../shared/api/types';
import { Breadcrumbs } from '../../../shared/components/Breadcrumbs/Breadcrumbs';
import { Page } from '../../../shared/components/Page/Page';
import { SettingsCard } from '../../../shared/components/SettingsCard/SettingsCard';
import { SettingsHeader } from '../../../shared/components/SettingsHeader/SettingsHeader';
import { SettingsLayout } from '../../../shared/components/SettingsLayout/SettingsLayout';
import { isPresent } from '../../../shared/defguard-ui/utils/isPresent';
import { getSettingsQueryOptions } from '../../../shared/query';
import { useQuery } from '@tanstack/react-query';

const breadcrumbs = [
  <Link
    to="/settings"
    search={{
      tab: 'general',
    }}
    key={0}
  >
    {m.settings_breadcrumb_general()}
  </Link>,
  <Link to="/settings/ca" key={1}>
    {m.settings_breadcrumb_instance()}
  </Link>,
];

export const SettingsCaPage = () => {
  const { data: settings } = useQuery(getSettingsQueryOptions);
  return (
    <Page title={m.settings_page_title()}>
      <Breadcrumbs links={breadcrumbs} />
      <SettingsLayout>
        <SettingsHeader
          icon="customize"
          title={m.settings_certs_ca_title()}
          subtitle={m.settings_certs_ca_description()}
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
  console.log(settings);
  return (<div>TODO</div>);
};
