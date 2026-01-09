import { createFileRoute } from '@tanstack/react-router';
import { CERulePage } from '../../../../pages/CERulePage/CERulePage';

export const Route = createFileRoute('/_authorized/_default/acl/add-rule')({
  component: CERulePage,
});
