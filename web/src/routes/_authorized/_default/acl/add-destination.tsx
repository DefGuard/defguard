import { createFileRoute, useSearch } from '@tanstack/react-router';
import { CEDestinationPage } from '../../../../pages/CEDestinationPage/CEDestinationPage';
import { aclFlowRouteSearchSchema } from '../../../../shared/aclTabs';

export const Route = createFileRoute('/_authorized/_default/acl/add-destination')({
  validateSearch: aclFlowRouteSearchSchema,
  component: RouteComponent,
});

function RouteComponent() {
  const search = useSearch({ from: '/_authorized/_default/acl/add-destination' });

  return <CEDestinationPage tab={search.tab} />;
}
