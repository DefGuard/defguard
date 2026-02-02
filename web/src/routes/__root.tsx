import type { QueryClient } from '@tanstack/react-query';
import { createRootRouteWithContext, Outlet, redirect } from '@tanstack/react-router';
import { isAxiosError } from 'axios';
import { queryClient } from '../app/query';
import { AppLoaderPage } from '../pages/AppLoaderPage/AppLoaderPage';
import type { User } from '../shared/api/types';
import { SnackbarManager } from '../shared/defguard-ui/providers/snackbar/SnackbarManager';
import { useAuth } from '../shared/hooks/useAuth';
import { getUserMeQueryOptions } from '../shared/query';

interface RouterContext {
  queryClient: QueryClient;
}

export const Route = createRootRouteWithContext<RouterContext>()({
  component: RootComponent,
  beforeLoad: async ({ location, context }) => {
    // only auto check for auth state if route is not in /auth flow
    if (location.pathname.startsWith('/auth')) {
      return;
    }
    // load logged in user info, ideally from query cache but if it's not there then ask API
    let user: User | null = null;
    const queryData = context.queryClient.getQueryData(getUserMeQueryOptions.queryKey);
    try {
      if (queryData) {
        const userMe = (await queryData()).data;
        user = userMe;
      } else {
        // Ensure query so the provider wrapper doesn't duplicate this request for no reason.
        const userMe = (
          await (
            await queryClient.ensureQueryData(getUserMeQueryOptions)
          )()
        ).data;
        user = userMe;
      }
    } catch (e) {
      if (isAxiosError(e) && e.status && e.status === 401) {
        useAuth.getState().reset();
        throw redirect({ to: '/auth/login', replace: true });
      }
    }
    if (user) {
      // Session is valid so redirect where it should land.
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
    }
  },
  pendingComponent: AppLoaderPage,
  pendingMs: 100,
});

function RootComponent() {
  return (
    <SnackbarManager>
      <Outlet />
    </SnackbarManager>
  );
}
