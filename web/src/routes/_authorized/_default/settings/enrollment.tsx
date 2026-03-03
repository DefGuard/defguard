import { createFileRoute } from '@tanstack/react-router';
import { SettingsEnrollmentPage } from '../../../../pages/settings/SettingsEnrollmentPage/SettingsEnrollmentPage';

export const Route = createFileRoute('/_authorized/_default/settings/enrollment')({
  component: SettingsEnrollmentPage,
});
