import { createFileRoute, useLoaderData, useSearch } from '@tanstack/react-router';
import { CEAliasPage } from '../../../../pages/CEAliasPage/CEAliasPage';
import { aclFlowRouteSearchSchema } from '../../../../shared/aclTabs';
import api from '../../../../shared/api/api';

export const Route = createFileRoute('/_authorized/_default/acl/add-alias')({
  validateSearch: aclFlowRouteSearchSchema,
  loaderDeps: ({ search }) => ({ search }),
  loader: async ({ deps: { search } }) => {
    if (search.duplicate === undefined) return;
    return (await api.acl.alias.getAlias(search.duplicate)).data;
  },
  component: RouteComponent,
});

function RouteComponent() {
  const search = useSearch({ from: '/_authorized/_default/acl/add-alias' });
  const duplicate = useLoaderData({ from: '/_authorized/_default/acl/add-alias' });

  return <CEAliasPage duplicate={duplicate} tab={search.tab} />;
}
