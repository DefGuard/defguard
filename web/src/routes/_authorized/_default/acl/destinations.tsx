import { createFileRoute } from '@tanstack/react-router';
import { DestinationsPage } from '../../../../pages/DestinationsPage/DestinationsPage';

export const Route = createFileRoute('/_authorized/_default/acl/destinations')({
  component: DestinationsPage,
});
