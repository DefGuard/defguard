import { createFileRoute, Outlet, redirect } from '@tanstack/react-router';
import { isPresent } from '../../shared/defguard-ui/utils/isPresent';
import { useAuth } from '../../shared/hooks/useAuth';

export const Route = createFileRoute('/auth/')({
  beforeLoad: () => {
    const authState = useAuth.getState();
    if (isPresent(authState.user)) {
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
