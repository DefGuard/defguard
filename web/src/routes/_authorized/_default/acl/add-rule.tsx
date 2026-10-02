import { createFileRoute, useLoaderData, useSearch } from '@tanstack/react-router';
import { CERulePage } from '../../../../pages/CERulePage/CERulePage';
import { aclFlowRouteSearchSchema } from '../../../../shared/aclTabs';
import api from '../../../../shared/api/api';

export const Route = createFileRoute('/_authorized/_default/acl/add-rule')({
  validateSearch: aclFlowRouteSearchSchema,
  loaderDeps: ({ search }) => ({ search }),
  loader: async ({ deps: { search } }) => {
    if (search.duplicate === undefined) return;
    return (await api.acl.rule.getRule(search.duplicate)).data;
  },
  component: RouteComponent,
});

function RouteComponent() {
  const search = useSearch({ from: '/_authorized/_default/acl/add-rule' });
  const duplicate = useLoaderData({ from: '/_authorized/_default/acl/add-rule' });

  return <CERulePage duplicate={duplicate} tab={search.tab} />;
}
