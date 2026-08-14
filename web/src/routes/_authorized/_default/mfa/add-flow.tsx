import { createFileRoute } from '@tanstack/react-router';
import { MfaFlowPage } from '../../../../pages/MfaPage/MfaFlowPage';

export const Route = createFileRoute('/_authorized/_default/mfa/add-flow')({
  component: MfaFlowPage,
});
