import { createFileRoute } from '@tanstack/react-router';
import z from 'zod';
import { LocationOverviewPage } from '../../../../pages/LocationOverviewPage/LocationOverviewPage';
import { getLocationQueryOptions } from '../../../../shared/query';

const vpnOverviewSearchSchema = z.object({
  period: z.number().int().default(1),
});

export const Route = createFileRoute('/_authorized/_default/vpn-overview/$locationId')({
  component: LocationOverviewPage,
  validateSearch: vpnOverviewSearchSchema,
  loader: ({ context, params }) => {
    return context.queryClient.ensureQueryData(
      getLocationQueryOptions(parseInt(params.locationId, 10)),
    );
  },
});
