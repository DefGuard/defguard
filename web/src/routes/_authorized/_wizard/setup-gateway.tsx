import { createFileRoute } from '@tanstack/react-router';
import { GatewaySetupPage } from '../../../pages/GatewaySetupPage/GatewaySetupPage';

export const Route = createFileRoute('/_authorized/_wizard/setup-gateway')({
  component: GatewaySetupPage,
});
