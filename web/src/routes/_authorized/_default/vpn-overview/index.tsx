import { createFileRoute } from '@tanstack/react-router';
import z from 'zod';
import { LocationsOverviewPage } from '../../../../pages/LocationsOverviewPage/LocationsOverviewPage';
import { getLocationsQueryOptions } from '../../../../shared/query';

const searchSchema = z.object({
  period: z.number().int().default(1),
});

export const Route = createFileRoute('/_authorized/_default/vpn-overview/')({
  component: LocationsOverviewPage,
  validateSearch: searchSchema,
  loader: ({ context }) => {
    return context.queryClient.ensureQueryData(getLocationsQueryOptions);
  },
});
