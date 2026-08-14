import { createFileRoute } from '@tanstack/react-router';
import { MfaPage } from '../../../../pages/MfaPage/MfaPage';

export const Route = createFileRoute('/_authorized/_default/mfa/')({
  component: MfaPage,
});
