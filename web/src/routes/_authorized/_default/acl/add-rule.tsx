import { createFileRoute, useSearch } from '@tanstack/react-router';
import { CERulePage } from '../../../../pages/CERulePage/CERulePage';
import { aclFlowRouteSearchSchema } from '../../../../shared/aclTabs';

export const Route = createFileRoute('/_authorized/_default/acl/add-rule')({
  validateSearch: aclFlowRouteSearchSchema,
  component: RouteComponent,
});

function RouteComponent() {
  const search = useSearch({ from: '/_authorized/_default/acl/add-rule' });

  return <CERulePage tab={search.tab} />;
}
