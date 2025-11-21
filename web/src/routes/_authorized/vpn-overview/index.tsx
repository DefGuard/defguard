import { createFileRoute } from '@tanstack/react-router';
import { LocationsOverviewPage } from '../../../pages/LocationsOverviewPage/LocationsOverviewPage';
import { getLocationsQueryOptions } from '../../../shared/query';

export const Route = createFileRoute('/_authorized/vpn-overview/')({
  component: LocationsOverviewPage,
  loader: ({ context }) => {
    return context.queryClient.ensureQueryData(getLocationsQueryOptions);
  },
});
