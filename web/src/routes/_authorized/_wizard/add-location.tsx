import { createFileRoute } from '@tanstack/react-router';
import { AddLocationPage } from '../../../pages/AddLocationPage/AddLocationPage';

export const Route = createFileRoute('/_authorized/_wizard/add-location')({
  component: AddLocationPage,
});
