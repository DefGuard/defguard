import { createFileRoute } from '@tanstack/react-router';
import { TrafficMonitoringPage } from '../../../pages/TrafficMonitoringPage/TrafficMonitoringPage';

export const Route = createFileRoute('/_authorized/_default/traffic')({
  component: TrafficMonitoringPage,
});
