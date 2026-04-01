import { Link } from '@tanstack/react-router';
import { m } from '../../../paraglide/messages';
import { Breadcrumbs } from '../../../shared/components/Breadcrumbs/Breadcrumbs';
import { Page } from '../../../shared/components/Page/Page';
import { SettingsCard } from '../../../shared/components/SettingsCard/SettingsCard';
import { SettingsHeader } from '../../../shared/components/SettingsHeader/SettingsHeader';
import { SettingsLayout } from '../../../shared/components/SettingsLayout/SettingsLayout';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import { MarkedSection } from '../../../shared/defguard-ui/components/MarkedSection/MarkedSection';
import { MarkedSectionHeader } from '../../../shared/defguard-ui/components/MarkedSectionHeader/MarkedSectionHeader';
import { Divider } from '../../../shared/defguard-ui/components/Divider/Divider';

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
          <Content />
        </SettingsCard>
      </SettingsLayout>
    </Page>
  );
};

const Content = () => {
  return (
    <>
      <MarkedSection icon="authorised-app">
        <MarkedSectionHeader
          title={m.settings_certs_certs_core_title()}
          description=''
        />
      </MarkedSection>
      <Divider spacing={ThemeSpacing.Xl2} />
      <MarkedSection icon="globe">
        <MarkedSectionHeader
          title={m.settings_instance_section_core()}
          description=''
        />
      </MarkedSection>
    </>
  );
};
