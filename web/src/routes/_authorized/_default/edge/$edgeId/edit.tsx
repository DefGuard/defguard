import { createFileRoute } from '@tanstack/react-router';
import { EdgeEditPage } from '../../../../../pages/EdgeEditPage/EdgeEditPage';
import { getEdgeQueryOptions } from '../../../../../shared/query';

export const Route = createFileRoute('/_authorized/_default/edge/$edgeId/edit')({
  loader: async ({ context, params }) => {
    const parsedId = parseInt(params.edgeId, 10);
    return context.queryClient.ensureQueryData(getEdgeQueryOptions(parsedId));
  },
  component: EdgeEditPage,
});
