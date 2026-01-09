import { createFileRoute } from '@tanstack/react-router';
import { EditLocationPage } from '../../../../../pages/EditLocationPage/EditLocationPage';
import { getLocationQueryOptions } from '../../../../../shared/query';

export const Route = createFileRoute('/_authorized/_default/locations/$locationId/edit')({
  component: EditLocationPage,
  loader: ({ context, params }) => {
    const parsedId = parseInt(params.locationId, 10);
    return context.queryClient.ensureQueryData(getLocationQueryOptions(parsedId));
  },
});
