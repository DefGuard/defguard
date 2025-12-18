import { createFileRoute } from '@tanstack/react-router';
import { RulesPage } from '../../../../pages/RulesPage/RulesPage';

export const Route = createFileRoute('/_authorized/_default/acl/rules')({
  component: RulesPage,
});
