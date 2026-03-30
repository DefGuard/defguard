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
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';

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
  <Link to="/settings/certs" key={1}>
    {m.settings_breadcrumb_instance()}
  </Link>,
];

export const SettingsCertificatesPage = () => {
  const { data: settings } = useQuery(getSettingsQueryOptions);
  return (
    <Page title={m.settings_page_title()}>
      <Breadcrumbs links={breadcrumbs} />
      <SettingsLayout>
        <SettingsHeader
          icon="customize"
          title={m.settings_certs_certs_title()}
          subtitle={m.settings_certs_certs_description()}
        />
        <SettingsCard>
          <SizedBox height={ThemeSpacing.Xl2} />
          <Content />
        </SettingsCard>
      </SettingsLayout>
    </Page>
  );
};

const Content = () => {
  return (<div>TODO</div>);
};
