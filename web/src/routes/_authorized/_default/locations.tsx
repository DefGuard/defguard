import { createFileRoute } from '@tanstack/react-router';
import { LocationsPage } from '../../../pages/LocationsPage/LocationsPage';
import { getLocationsQueryOptions } from '../../../shared/query';

export const Route = createFileRoute('/_authorized/_default/locations')({
  component: LocationsPage,
  loader: ({ context }) => {
    return context.queryClient.ensureQueryData(getLocationsQueryOptions);
  },
});
