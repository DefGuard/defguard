import { useQuery } from '@tanstack/react-query';
import { Link, useNavigate } from '@tanstack/react-router';
import { useCallback } from 'react';
import { m } from '../../../paraglide/messages';
import api from '../../../shared/api/api';
import { Breadcrumbs } from '../../../shared/components/Breadcrumbs/Breadcrumbs';
import { DescriptionBlock } from '../../../shared/components/DescriptionBlock/DescriptionBlock';
import { Page } from '../../../shared/components/Page/Page';
import { SettingsCard } from '../../../shared/components/SettingsCard/SettingsCard';
import { SettingsHeader } from '../../../shared/components/SettingsHeader/SettingsHeader';
import { SettingsLayout } from '../../../shared/components/SettingsLayout/SettingsLayout';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { Divider } from '../../../shared/defguard-ui/components/Divider/Divider';
import { MarkedSection } from '../../../shared/defguard-ui/components/MarkedSection/MarkedSection';
import { MarkedSectionHeader } from '../../../shared/defguard-ui/components/MarkedSectionHeader/MarkedSectionHeader';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import { isPresent } from '../../../shared/defguard-ui/utils/isPresent';
import { downloadFile } from '../../../shared/utils/download';

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
  const navigate = useNavigate();
  const { data: certsData, isFetching } = useQuery({
    queryKey: ['core', 'cert', 'certs'],
    queryFn: api.core.getCerts,
    select: (resp) => resp.data,
  });
  const { data: caData, isFetching: isFetchingCa } = useQuery({
    queryKey: ['core', 'cert', 'ca'],
    queryFn: api.core.getCA,
    select: (resp) => resp.data,
  });

  const downloadCaCert = useCallback(() => {
    const caPem = caData?.ca_cert_pem;
    if (!isPresent(caPem)) return;
    const blob = new Blob([caPem], {
      type: 'application/x-pem-file;charset=utf-8',
    });
    downloadFile(blob, 'defguard-ca', 'pem');
  }, [caData?.ca_cert_pem]);

  return (
    <>
      <MarkedSection icon="authorised-app">
        <MarkedSectionHeader title={m.settings_certs_certs_core_title()} />
        {certsData?.core_http_cert_source === 'None' && (
          <>
            <DescriptionBlock title={m.settings_certs_certs_none_title()}>
              <p>{m.settings_certs_certs_none_description()}</p>
            </DescriptionBlock>
            <SizedBox height={ThemeSpacing.Lg} />
            <Button
              variant="primary"
              text={m.settings_certs_certs_none_add_certificate()}
              onClick={() => void navigate({ to: '/settings-core-certificate' })}
              loading={false}
              disabled={false}
            />
          </>
        )}
        {certsData?.core_http_cert_source === 'SelfSigned' && (
          <>
            <DescriptionBlock title={m.settings_certs_certs_internal_title()}>
              <p>{m.settings_certs_certs_internal_description()}</p>
            </DescriptionBlock>
            <SizedBox height={ThemeSpacing.Lg} />
            <Button
              variant="outlined"
              text={m.settings_certs_ca_download()}
              onClick={downloadCaCert}
              loading={isFetching || isFetchingCa}
              disabled={!isPresent(caData?.ca_cert_pem) || isFetching || isFetchingCa}
            />
          </>
        )}
        {certsData?.core_http_cert_source === 'Custom' && (
          <>
            <DescriptionBlock title={m.settings_certs_certs_custom_title()}>
              <p>{m.settings_certs_certs_custom_description()}</p>
            </DescriptionBlock>
            <SizedBox height={ThemeSpacing.Lg} />
            <Button
              variant="primary"
              text={m.settings_certs_certs_custom_change()}
              onClick={() => void navigate({ to: '/settings-core-certificate' })}
              loading={false}
              disabled={false}
            />
          </>
        )}
      </MarkedSection>
      <Divider spacing={ThemeSpacing.Xl2} />
      <MarkedSection icon="globe">
        <MarkedSectionHeader title={m.settings_certs_certs_edge_title()} />
        {certsData?.proxy_http_cert_source === 'None' && (
          <>
            <DescriptionBlock title={m.settings_certs_certs_none_title()}>
              <p>{m.settings_certs_certs_none_description()}</p>
            </DescriptionBlock>
            <SizedBox height={ThemeSpacing.Lg} />
            <Button
              variant="primary"
              text={m.settings_certs_certs_none_add_certificate()}
              onClick={() => void navigate({ to: '/settings-edge-certificate' })}
              loading={false}
              disabled={false}
            />
          </>
        )}
        {certsData?.proxy_http_cert_source === 'SelfSigned' && (
          <>
            <DescriptionBlock title={m.settings_certs_certs_internal_title()}>
              <p>{m.settings_certs_certs_internal_description()}</p>
            </DescriptionBlock>
            <SizedBox height={ThemeSpacing.Lg} />
            <Button
              variant="outlined"
              text={m.settings_certs_ca_download()}
              onClick={downloadCaCert}
              loading={isFetching || isFetchingCa}
              disabled={!isPresent(caData?.ca_cert_pem) || isFetching || isFetchingCa}
            />
          </>
        )}
        {certsData?.proxy_http_cert_source === 'Custom' && (
          <>
            <DescriptionBlock title={m.settings_certs_certs_custom_title()}>
              <p>{m.settings_certs_certs_custom_description()}</p>
            </DescriptionBlock>
            <SizedBox height={ThemeSpacing.Lg} />
            <Button
              variant="primary"
              text={m.settings_certs_certs_custom_change()}
              onClick={() => void navigate({ to: '/settings-edge-certificate' })}
              loading={false}
              disabled={false}
            />
          </>
        )}
      </MarkedSection>
    </>
  );
};
