import { createFileRoute, useSearch } from '@tanstack/react-router';
import { CEAliasPage } from '../../../../pages/CEAliasPage/CEAliasPage';
import { aclFlowRouteSearchSchema } from '../../../../shared/aclTabs';

export const Route = createFileRoute('/_authorized/_default/acl/add-alias')({
  validateSearch: aclFlowRouteSearchSchema,
  component: RouteComponent,
});

function RouteComponent() {
  const search = useSearch({ from: '/_authorized/_default/acl/add-alias' });

  return <CEAliasPage tab={search.tab} />;
}
