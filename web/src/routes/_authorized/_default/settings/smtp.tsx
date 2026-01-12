import { createFileRoute } from '@tanstack/react-router';
import { SettingsSmtpPage } from '../../../../pages/settings/SettingsSmtpPage/SettingsSmtpPage';

export const Route = createFileRoute('/_authorized/_default/settings/smtp')({
  component: SettingsSmtpPage,
});
