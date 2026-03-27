import { createFileRoute, useLoaderData } from '@tanstack/react-router';
import z from 'zod';
import { CERulePage } from '../../../../pages/CERulePage/CERulePage';
import { aclListTabSchema } from '../../../../shared/aclTabs';
import api from '../../../../shared/api/api';

const searchSchema = z.object({
  rule: z.number(),
  tab: aclListTabSchema.optional(),
});

export const Route = createFileRoute('/_authorized/_default/acl/edit-rule')({
  validateSearch: searchSchema,
  loaderDeps: ({ search }) => ({ search }),
  loader: async ({ deps: { search } }) => {
    return (await api.acl.rule.getRule(search.rule)).data;
  },
  component: RouteComponent,
});

function RouteComponent() {
  const rule = useLoaderData({ from: '/_authorized/_default/acl/edit-rule' });
  const search = Route.useSearch();

  return <CERulePage rule={rule} tab={search.tab} />;
}
