import { createFileRoute } from '@tanstack/react-router';
import { LoginRecovery } from '../../../pages/auth/LoginRecovery/LoginRecovery';

export const Route = createFileRoute('/auth/mfa/recovery')({
  component: LoginRecovery,
});
