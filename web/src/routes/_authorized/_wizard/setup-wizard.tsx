import { createFileRoute } from '@tanstack/react-router';
import { SetupPage } from '../../../pages/SetupPage/SetupPage';

export const Route = createFileRoute('/_authorized/_wizard/setup-wizard')({
  component: SetupPage,
});
