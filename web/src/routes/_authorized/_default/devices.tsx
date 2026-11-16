import { createFileRoute } from '@tanstack/react-router';
import { DevicesPage } from '../../../pages/DevicesPage/DevicesPage';

export const Route = createFileRoute('/_authorized/_default/devices')({
  component: DevicesPage,
});
