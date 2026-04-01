import { Link } from '@tanstack/react-router';
import { m } from '../../../paraglide/messages';
import { Breadcrumbs } from '../../../shared/components/Breadcrumbs/Breadcrumbs';
import { Page } from '../../../shared/components/Page/Page';
import { SettingsCard } from '../../../shared/components/SettingsCard/SettingsCard';
import { SettingsHeader } from '../../../shared/components/SettingsHeader/SettingsHeader';
import { SettingsLayout } from '../../../shared/components/SettingsLayout/SettingsLayout';
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
  return (
    <Page title={m.settings_page_title()}>
      <Breadcrumbs links={breadcrumbs} />
      <SettingsLayout>
        <SettingsHeader
          icon="aliases"
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
  return <div>TODO</div>;
};
