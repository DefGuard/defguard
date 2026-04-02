import './style.scss';
import { useQuery } from '@tanstack/react-query';
import { Link } from '@tanstack/react-router';
import { useCallback } from 'react';
import { m } from '../../../paraglide/messages';
import api from '../../../shared/api/api';
import { Breadcrumbs } from '../../../shared/components/Breadcrumbs/Breadcrumbs';
import { DescriptionBlock } from '../../../shared/components/DescriptionBlock/DescriptionBlock';
import { Page } from '../../../shared/components/Page/Page';
import { SettingsCard } from '../../../shared/components/SettingsCard/SettingsCard';
import { SettingsHeader } from '../../../shared/components/SettingsHeader/SettingsHeader';
import { SettingsLayout } from '../../../shared/components/SettingsLayout/SettingsLayout';
import { ActionableSection } from '../../../shared/defguard-ui/components/ActionableSection/ActionableSection';
import { ActionableSectionVariant } from '../../../shared/defguard-ui/components/ActionableSection/types';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { Divider } from '../../../shared/defguard-ui/components/Divider/Divider';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import { isPresent } from '../../../shared/defguard-ui/utils/isPresent';
import { displayDate } from '../../../shared/utils/displayDate';
import { downloadFile } from '../../../shared/utils/download';
import caIconSrc from './assets/ca.png';

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
          icon="aliases"
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
  const { data: caData, isFetching } = useQuery({
    queryKey: ['core', 'cert', 'ca'],
    queryFn: api.core.getCA,
    select: (resp) => resp.data,
  });

  const handleDownloadCA = useCallback(() => {
    const caPem = caData?.ca_cert_pem;
    if (!isPresent(caPem)) return;
    const blob = new Blob([caPem], {
      type: 'application/x-pem-file;charset=utf-8',
    });
    downloadFile(blob, 'defguard-ca', 'pem');
  }, [caData?.ca_cert_pem]);

  return (
    <ActionableSection
      variant={ActionableSectionVariant.Secondary}
      title={m.settings_certs_ca_certificate_validated_title()}
      subtitle={m.settings_certs_ca_certificate_validated()}
      imageSrc={caIconSrc}
    >
      <SizedBox height={ThemeSpacing.Xl3} />
      <p className="ca-info-title">{m.settings_certs_ca_information_extracted()}</p>
      <Divider spacing={ThemeSpacing.Md} />
      <div className="ca-info-grid">
        <div className="ca-info-label">{m.settings_certs_ca_email()}</div>
        <div className="ca-info-value">{caData?.subject_email}</div>
        <div className="ca-info-label">{m.settings_certs_ca_valid_until()}</div>
        <div className="ca-info-value">
          {caData?.ca_expiry ? displayDate(caData?.ca_expiry) : '-'}
        </div>
      </div>
      <Divider spacing={ThemeSpacing.Md} />
      <SizedBox height={ThemeSpacing.Xl2} />
      <Button
        variant="outlined"
        iconLeft="download"
        text={m.settings_certs_ca_download()}
        onClick={handleDownloadCA}
        loading={isFetching}
        disabled={!isPresent(caData?.ca_cert_pem) || isFetching}
      />
    </ActionableSection>
  );
};
