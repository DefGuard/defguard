import { createFileRoute, useLoaderData } from '@tanstack/react-router';
import z from 'zod';
import { CEDestinationPage } from '../../../../pages/CEDestinationPage/CEDestinationPage';
import api from '../../../../shared/api/api';

const searchSchema = z.object({
  destination: z.number(),
});

export const Route = createFileRoute('/_authorized/_default/acl/edit-destination')({
  validateSearch: searchSchema,
  loaderDeps: ({ search }) => ({ search }),
  loader: async ({ deps }) => {
    const destination = (
      await api.acl.destination.getDestination(deps.search.destination)
    ).data;
    return destination;
  },
  component: RouteComponent,
});

function RouteComponent() {
  const destination = useLoaderData({
    from: '/_authorized/_default/acl/edit-destination',
  });
  return <CEDestinationPage destination={destination} />;
}
