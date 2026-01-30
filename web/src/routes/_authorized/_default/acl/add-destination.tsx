import { createFileRoute } from '@tanstack/react-router';
import { CEDestinationPage } from '../../../../pages/CEDestinationPage/CEDestinationPage';

export const Route = createFileRoute('/_authorized/_default/acl/add-destination')({
  component: CEDestinationPage,
});
