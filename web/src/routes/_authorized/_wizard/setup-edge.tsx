import { createFileRoute } from '@tanstack/react-router';
import { EdgeSetupPage } from '../../../pages/EdgeSetupPage/EdgeSetupPage';

export const Route = createFileRoute('/_authorized/_wizard/setup-edge')({
  component: EdgeSetupPage,
});
