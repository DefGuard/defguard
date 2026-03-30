import './style.scss';
import { Link } from '@tanstack/react-router';
import { m } from '../../../paraglide/messages';
import { Breadcrumbs } from '../../../shared/components/Breadcrumbs/Breadcrumbs';
import { Page } from '../../../shared/components/Page/Page';
import { SettingsCard } from '../../../shared/components/SettingsCard/SettingsCard';
import { SettingsHeader } from '../../../shared/components/SettingsHeader/SettingsHeader';
import { SettingsLayout } from '../../../shared/components/SettingsLayout/SettingsLayout';
import { DescriptionBlock } from '../../../shared/components/DescriptionBlock/DescriptionBlock';
import { ActionableSection } from '../../../shared/defguard-ui/components/ActionableSection/ActionableSection';
import { ActionableSectionVariant } from '../../../shared/defguard-ui/components/ActionableSection/types';
import caIconSrc from '../../SetupPage/assets/ca.png';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import { Divider } from '../../../shared/defguard-ui/components/Divider/Divider';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';

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
  return (
    <Page title={m.settings_page_title()}>
      <Breadcrumbs links={breadcrumbs} />
      <SettingsLayout>
        <SettingsHeader
          icon="customize"
          title={m.settings_certs_ca_title()}
          subtitle={m.settings_certs_ca_description()}
        />
        <SettingsCard>
          <DescriptionBlock title={m.settings_certs_ca_summary_title()}>
            <p>{m.settings_certs_ca_summary()}</p>
          </DescriptionBlock>
          <SizedBox height={ThemeSpacing.Xl2} />
          <Content />
        </SettingsCard>
      </SettingsLayout>
    </Page>
  );
};

const Content = () => {
  return (
    <ActionableSection
      variant={ActionableSectionVariant.Secondary}
      title={m.settings_certs_ca_certificate_validated_title()}
      subtitle={m.settings_certs_ca_certificate_validated()}
      imageSrc={caIconSrc}
    >
      <SizedBox height={ThemeSpacing.Xl3} />
      <p className='ca-info-title'>{m.settings_certs_ca_information_extracted()}</p>
      <Divider spacing={ThemeSpacing.Md}/>
      <div className='ca-info-grid'>
        <div className='ca-info-label'>{m.settings_certs_ca_email()}</div>
        <div className='ca-info-value'>TODO</div>
        <div className='ca-info-label'>{m.settings_certs_ca_valid_until()}</div>
        <div className='ca-info-value'>TODO</div>
      </div>
      <Divider spacing={ThemeSpacing.Md}/>
      <SizedBox height={ThemeSpacing.Xl2} />
      <Button variant='outlined' text={m.settings_certs_ca_download()} iconLeft='download'/>
    </ActionableSection>
  );
};
