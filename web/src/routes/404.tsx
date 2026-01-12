import { createFileRoute, redirect } from '@tanstack/react-router';
import { isPresent } from '../shared/defguard-ui/utils/isPresent';
import { useAuth } from '../shared/hooks/useAuth';

export const Route = createFileRoute('/404')({
  beforeLoad: () => {
    const state = useAuth.getState();
    if (state.isAuthenticated && isPresent(state.user)) {
      if (state.user.is_admin) {
        throw redirect({
          to: '/vpn-overview',
          replace: true,
        });
      } else {
        throw redirect({
          to: '/user/$username',
          params: {
            username: state.user?.username,
          },
          replace: true,
        });
      }
    } else {
      throw redirect({ to: '/auth/login', replace: true });
    }
  },
  component: RouteComponent,
});

function RouteComponent() {
  return (
    <div>
      <p>Not found!</p>
    </div>
  );
}
