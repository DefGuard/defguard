import { createFileRoute } from '@tanstack/react-router';
import { EditGatewayPage } from '../../../../../pages/EditGatewayPage/EditGatewayPage';
import { getGatewayQueryOptions } from '../../../../../shared/query';

export const Route = createFileRoute('/_authorized/_default/gateway/$gatewayId/edit')({
  loader: async ({ context, params }) => {
    const parsedId = parseInt(params.gatewayId, 10);
    return context.queryClient.ensureQueryData(getGatewayQueryOptions(parsedId));
  },
  component: EditGatewayPage,
});
