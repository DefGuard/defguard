import { createFileRoute } from '@tanstack/react-router';
import { TotpLogin } from '../../../pages/auth/TotpLogin/TotpLogin';

export const Route = createFileRoute('/auth/mfa/totp')({
  component: TotpLogin,
});
