import { createFileRoute } from '@tanstack/react-router';
import { LoginEmail } from '../../../pages/auth/LoginEmail/LoginEmail';

export const Route = createFileRoute('/auth/mfa/email')({
  component: LoginEmail,
});
