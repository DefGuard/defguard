import { createFileRoute } from '@tanstack/react-router'
import { SettingsCertificatesPage } from '../../../../pages/settings/SettingsCertificatesPage/SettingsCertificatesPage';

export const Route = createFileRoute('/_authorized/_default/settings/certs')({
  component: SettingsCertificatesPage,
});
