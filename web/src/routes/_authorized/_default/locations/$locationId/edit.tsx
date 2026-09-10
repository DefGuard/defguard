import { createFileRoute } from '@tanstack/react-router';
import { EditLocationPage } from '../../../../../pages/EditLocationPage/EditLocationPage';
import {
  getLocationMfaFlowsQueryOptions,
  getLocationQueryOptions,
  getMfaFlowsQueryOptions,
} from '../../../../../shared/query';

export const Route = createFileRoute('/_authorized/_default/locations/$locationId/edit')({
  component: EditLocationPage,
  loader: async ({ context, params }) => {
    const parsedId = parseInt(params.locationId, 10);

    await Promise.all([
      context.queryClient.query(getLocationQueryOptions(parsedId)),
      context.queryClient.query(getLocationMfaFlowsQueryOptions(parsedId)),
      context.queryClient.query(getMfaFlowsQueryOptions),
    ]);
  },
});
