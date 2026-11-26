import { createFileRoute } from '@tanstack/react-router';
import { SettingsDeviceAttestationPage } from '../../../../pages/settings/SettingsDeviceAttestationPage/SettingsDeviceAttestationPage';

export const Route = createFileRoute('/_authorized/_default/settings/device-attestation')(
  {
    component: SettingsDeviceAttestationPage,
  },
);
