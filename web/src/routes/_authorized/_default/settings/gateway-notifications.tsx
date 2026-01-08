import { createFileRoute } from '@tanstack/react-router';
import { SettingsGatewayNotificationsPage } from '../../../../pages/settings/SettingsExternalOpenIdPage/SettingsGatewayNotificationsPage/SettingsGatewayNotificationsPage';

export const Route = createFileRoute(
  '/_authorized/_default/settings/gateway-notifications',
)({
  component: SettingsGatewayNotificationsPage,
});
