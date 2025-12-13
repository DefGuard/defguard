import { createFileRoute } from '@tanstack/react-router';
import { SettingsInstancePage } from '../../../../pages/settings/SettingsInstancePage/SettingsInstancePage';

export const Route = createFileRoute('/_authorized/_default/settings/instance')({
  component: SettingsInstancePage,
});
