import { createFileRoute } from '@tanstack/react-router';
import { CEAliasPage } from '../../../../pages/CEAliasPage/CEAliasPage';

export const Route = createFileRoute('/_authorized/_default/acl/add-alias')({
  component: RouteComponent,
});

function RouteComponent() {
  return <CEAliasPage />;
}
