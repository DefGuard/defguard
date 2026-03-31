import { createFileRoute } from '@tanstack/react-router';
import { SettingsCaPage } from '../../../../pages/settings/SettingsCaPage/SettingsCaPage';

export const Route = createFileRoute('/_authorized/_default/settings/ca')({
  component: SettingsCaPage,
});
