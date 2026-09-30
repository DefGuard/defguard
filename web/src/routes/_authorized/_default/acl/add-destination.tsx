import { createFileRoute, useLoaderData, useSearch } from '@tanstack/react-router';
import { CEDestinationPage } from '../../../../pages/CEDestinationPage/CEDestinationPage';
import { aclFlowRouteSearchSchema } from '../../../../shared/aclTabs';
import api from '../../../../shared/api/api';

export const Route = createFileRoute('/_authorized/_default/acl/add-destination')({
  validateSearch: aclFlowRouteSearchSchema,
  loaderDeps: ({ search }) => ({ search }),
  loader: async ({ deps: { search } }) => {
    if (search.duplicate === undefined) return;
    return (await api.acl.destination.getDestination(search.duplicate)).data;
  },
  component: RouteComponent,
});

function RouteComponent() {
  const search = useSearch({ from: '/_authorized/_default/acl/add-destination' });
  const duplicate = useLoaderData({ from: '/_authorized/_default/acl/add-destination' });

  return <CEDestinationPage duplicate={duplicate} tab={search.tab} />;
}
