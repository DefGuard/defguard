import { createFileRoute } from '@tanstack/react-router';
import { LoginLoadingPage } from '../../pages/auth/LoginLoading/LoginLoadingPage';

export const Route = createFileRoute('/auth/loading')({
  component: LoginLoadingPage,
});
