import { createFileRoute } from '@tanstack/react-router';
import { LoginWebauthn } from '../../../pages/auth/LoginWebauthn/LoginWebauthn';

export const Route = createFileRoute('/auth/mfa/webauthn')({
  component: LoginWebauthn,
});
