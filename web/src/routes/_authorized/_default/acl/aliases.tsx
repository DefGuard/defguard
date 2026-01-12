import { createFileRoute } from '@tanstack/react-router';
import { AliasesPage } from '../../../../pages/AliasesPage/AliasesPage';

export const Route = createFileRoute('/_authorized/_default/acl/aliases')({
  component: AliasesPage,
});
