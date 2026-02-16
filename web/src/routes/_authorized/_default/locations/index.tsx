import { createFileRoute } from '@tanstack/react-router';
import { LocationsPage } from '../../../../pages/LocationsPage/LocationsPage';

export const Route = createFileRoute('/_authorized/_default/locations/')({
  component: LocationsPage,
});
