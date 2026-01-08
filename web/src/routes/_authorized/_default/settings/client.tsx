import { createFileRoute } from '@tanstack/react-router';
import { SettingsClientPage } from '../../../../pages/settings/SettingsClientPage/SettingsClientPage';

export const Route = createFileRoute('/_authorized/_default/settings/client')({
  component: SettingsClientPage,
});
