import { createFileRoute } from '@tanstack/react-router';
import { LoginMainPage } from '../../pages/auth/LoginMain/LoginMainPage';

export const Route = createFileRoute('/auth/login')({
  component: LoginMainPage,
});
