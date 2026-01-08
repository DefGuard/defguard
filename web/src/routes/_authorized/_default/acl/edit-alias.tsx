import { createFileRoute, useLoaderData } from '@tanstack/react-router';
import z from 'zod';
import { CEAliasPage } from '../../../../pages/CEAliasPage/CEAliasPage';
import api from '../../../../shared/api/api';

const searchSchema = z.object({
  alias: z.number(),
});

export const Route = createFileRoute('/_authorized/_default/acl/edit-alias')({
  validateSearch: searchSchema,
  loaderDeps: ({ search }) => ({ search }),
  loader: async ({ deps }) => {
    const alias = (await api.acl.alias.getAlias(deps.search.alias)).data;
    return alias;
  },
  component: RouteComponent,
});

function RouteComponent() {
  const alias = useLoaderData({ from: '/_authorized/_default/acl/edit-alias' });
  return <CEAliasPage alias={alias} />;
}
