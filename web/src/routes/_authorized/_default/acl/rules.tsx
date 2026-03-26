import { createFileRoute } from '@tanstack/react-router';
import { RulesPage } from '../../../../pages/RulesPage/RulesPage';
import { aclListRouteSearchSchema } from '../../../../shared/aclTabs';

export const Route = createFileRoute('/_authorized/_default/acl/rules')({
  validateSearch: aclListRouteSearchSchema,
  component: RulesPage,
});
