import { Link } from '@tanstack/react-router';
import { m } from '../../../paraglide/messages';
import { Breadcrumbs } from '../../../shared/components/Breadcrumbs/Breadcrumbs';
import { Page } from '../../../shared/components/Page/Page';
import { SettingsHeader } from '../../../shared/components/SettingsHeader/SettingsHeader';
import { SettingsLayout } from '../../../shared/components/SettingsLayout/SettingsLayout';
import { DeviceAttestationCertificates } from './components/DeviceAttestationCertificates/DeviceAttestationCertificates';

const breadcrumbs = [
  <Link to="/settings" search={{ tab: 'general' }} key={0}>
    {m.settings_breadcrumb_general()}
  </Link>,
  <Link to="/settings/device-attestation" key={1}>
    {m.settings_breadcrumb_device_attestation()}
  </Link>,
];

export const SettingsDeviceAttestationPage = () => {
  return (
    <Page title={m.settings_page_title()}>
      <Breadcrumbs links={breadcrumbs} />
      <SettingsLayout>
        <SettingsHeader
          icon="device-attestation"
          title={m.settings_device_attestation_title()}
          subtitle={m.settings_device_attestation_subtitle()}
        />
        <DeviceAttestationCertificates />
      </SettingsLayout>
    </Page>
  );
};
