import { createFileRoute } from '@tanstack/react-router';
import { LocationOverviewPage } from '../../../pages/LocationOverviewPage/LocationOverviewPage';
import { getLocationQueryOptions } from '../../../shared/query';
import { vpnOverviewSearchSchema } from './_search';

export const Route = createFileRoute('/_authorized/vpn-overview/$locationId')({
  component: LocationOverviewPage,
  validateSearch: vpnOverviewSearchSchema,
  loader: ({ context, params }) => {
    return context.queryClient.ensureQueryData(
      getLocationQueryOptions(parseInt(params.locationId, 10)),
    );
  },
});
