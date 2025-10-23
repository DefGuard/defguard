import { createFileRoute, Outlet, redirect } from '@tanstack/react-router';
import { useAuth } from '../../shared/hooks/useAuth';

export const Route = createFileRoute('/auth/mfa')({
  beforeLoad: () => {
    const authState = useAuth.getState().mfaLogin;
    if (!authState) {
      throw redirect({
        to: '/auth/login',
        replace: true,
      });
    }
    return authState;
  },
  component: RouteComponent,
});

function RouteComponent() {
  return <Outlet />;
}
