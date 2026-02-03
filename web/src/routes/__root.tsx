import type { QueryClient } from '@tanstack/react-query';
import { createRootRouteWithContext, Outlet, redirect } from '@tanstack/react-router';
import { AppLoaderPage } from '../pages/AppLoaderPage/AppLoaderPage';
import { SnackbarManager } from '../shared/defguard-ui/providers/snackbar/SnackbarManager';
import { useAuth } from '../shared/hooks/useAuth';
import { getUserMeQueryOptions } from '../shared/query';

interface RouterContext {
  queryClient: QueryClient;
}

export const Route = createRootRouteWithContext<RouterContext>()({
  component: RootComponent,
  beforeLoad: async ({ location, context }) => {
    if (location.pathname.startsWith('/auth')) {
      return;
    }
    try {
      const user = (
        await (
          await context.queryClient.ensureQueryData(getUserMeQueryOptions)
        )()
      ).data;
      useAuth.getState().setUser(user);
    } catch (_) {
      useAuth.getState().reset();
      throw redirect({
        to: '/auth/login',
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
