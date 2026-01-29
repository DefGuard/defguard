import { createFileRoute } from '@tanstack/react-router';
import { EditEdgePage } from '../../../../../pages/EditEdgePage/EditEdgePage';
import { getEdgeQueryOptions } from '../../../../../shared/query';

export const Route = createFileRoute('/_authorized/_default/edge/$edgeId/edit')({
  loader: async ({ context, params }) => {
    const parsedId = parseInt(params.edgeId, 10);
    return context.queryClient.ensureQueryData(getEdgeQueryOptions(parsedId));
  },
  component: EditEdgePage,
});
