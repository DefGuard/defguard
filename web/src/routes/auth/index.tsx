import { createFileRoute, redirect } from '@tanstack/react-router';
import { LoginLoadingPage } from '../../pages/auth/LoginLoading/LoginLoadingPage';
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
    } else {
      throw redirect({
        to: '/auth/login',
        replace: true,
      });
    }
  },
  component: RouteComponent,
});

function RouteComponent() {
  return <LoginLoadingPage />;
}
