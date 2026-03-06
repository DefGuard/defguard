import { createFileRoute } from '@tanstack/react-router';
import { SettingsVpnStatsPage } from '../../../../pages/settings/SettingsVpnStatsPage/SettingsVpnStatsPage';

export const Route = createFileRoute('/_authorized/_default/settings/vpn-stats')({
  component: SettingsVpnStatsPage,
});
