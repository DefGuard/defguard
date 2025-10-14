import { createFileRoute, Outlet, redirect } from '@tanstack/react-router';
import { useAuth } from '../../shared/hooks/useAuth';

export const Route = createFileRoute('/auth/')({
  beforeLoad: () => {
    const authState = useAuth.getState();
    if (authState.isAuth && authState.user) {
      throw redirect({
        to: '/user/$username',
        params: {
          username: authState.user.username,
        },
      });
    }
  },
  component: RouteComponent,
});

function RouteComponent() {
  return <Outlet />;
}
