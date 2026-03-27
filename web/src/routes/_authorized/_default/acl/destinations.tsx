import { createFileRoute } from '@tanstack/react-router';
import { DestinationsPage } from '../../../../pages/DestinationsPage/DestinationsPage';
import { aclListRouteSearchSchema } from '../../../../shared/aclTabs';

export const Route = createFileRoute('/_authorized/_default/acl/destinations')({
  validateSearch: aclListRouteSearchSchema,
  component: DestinationsPage,
});
