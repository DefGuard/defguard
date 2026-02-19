import { createFileRoute } from '@tanstack/react-router';
import { SettingsExternalOpenIdPage } from '../../../../pages/settings/SettingsExternalOpenIdPage/SettingsExternalOpenIdPage';

export const Route = createFileRoute('/_authorized/_default/settings/openid')({
  component: SettingsExternalOpenIdPage,
});
