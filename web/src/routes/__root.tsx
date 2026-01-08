import type { QueryClient } from '@tanstack/react-query';
import { createRootRouteWithContext, Outlet, redirect } from '@tanstack/react-router';
import { AppLoaderPage } from '../pages/AppLoaderPage/AppLoaderPage';
import api from '../shared/api/api';
import { isPresent } from '../shared/defguard-ui/utils/isPresent';
import { useAuth } from '../shared/hooks/useAuth';

interface RouterContext {
  queryClient: QueryClient;
}

export const Route = createRootRouteWithContext<RouterContext>()({
  component: RootComponent,
  beforeLoad: async ({ location }) => {
    // only auto check for auth state if route is not in /auth flow
    if (location.pathname.startsWith('/auth')) {
      return;
    }
    if (!isPresent(useAuth.getState().user)) {
      try {
        const { data: user } = await api.user.getMe();
        useAuth.getState().setUser(user);
        if (user.is_admin) {
          throw redirect({ to: '/vpn-overview', replace: true });
        }
        throw redirect({
          to: '/user/$username',
          params: {
            username: user.username,
          },
          replace: true,
        });
      } catch (_) {}
    }
  },
  pendingComponent: AppLoaderPage,
  pendingMs: 100,
});

function RootComponent() {
  return <Outlet />;
}
