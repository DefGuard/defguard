import { createFileRoute } from '@tanstack/react-router';
import { SettingsEdgeCertificateWizardPage } from '../../../pages/settings/SettingsCertificatesPage/SettingsEdgeCertificateWizardPage/SettingsEdgeCertificateWizardPage';

export const Route = createFileRoute('/_authorized/_wizard/settings-edge-certificate')({
  component: SettingsEdgeCertificateWizardPage,
});
