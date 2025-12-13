import { createFileRoute } from '@tanstack/react-router';
import { SettingsExternalOpenidGeneralPage } from '../../../../../pages/settings/SettingsExternalOpenidGeneralPage/SettingsExternalOpenidGeneralPage';

export const Route = createFileRoute('/_authorized/_default/settings/openid/general')({
  component: SettingsExternalOpenidGeneralPage,
});
