import { createFileRoute } from '@tanstack/react-router';
import { MfaFormPage } from '../../../../pages/MfaPage/MfaFormPage';

export const Route = createFileRoute('/_authorized/_default/mfa/add-flow')({
  component: MfaFormPage,
});
