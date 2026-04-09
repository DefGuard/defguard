import { createFileRoute } from '@tanstack/react-router';
import { SettingsCoreCertificateWizardPage } from '../../../pages/SettingsCoreCertificateWizardPage/SettingsCoreCertificateWizardPage';

export const Route = createFileRoute('/_authorized/_wizard/settings-core-certificate')({
  component: SettingsCoreCertificateWizardPage,
});
