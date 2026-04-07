import { createFileRoute } from '@tanstack/react-router';
import { AliasesPage } from '../../../../pages/AliasesPage/AliasesPage';
import { aclListRouteSearchSchema } from '../../../../shared/aclTabs';

export const Route = createFileRoute('/_authorized/_default/acl/aliases')({
  validateSearch: aclListRouteSearchSchema,
  component: AliasesPage,
});
