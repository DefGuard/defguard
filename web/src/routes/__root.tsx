import type { QueryClient } from '@tanstack/react-query';
import { createRootRouteWithContext, Outlet, redirect } from '@tanstack/react-router';
import { AppLoaderPage } from '../pages/AppLoaderPage/AppLoaderPage';
import { useSetupWizardStore } from '../pages/SetupPage/useSetupWizardStore';
import api from '../shared/api/api';
import { SnackbarManager } from '../shared/defguard-ui/providers/snackbar/SnackbarManager';
import { isPresent } from '../shared/defguard-ui/utils/isPresent';
import { useAuth } from '../shared/hooks/useAuth';

interface RouterContext {
  queryClient: QueryClient;
}

export const Route = createRootRouteWithContext<RouterContext>()({
  component: RootComponent,
  beforeLoad: async ({ location }) => {
    const appInfo = await api.settings
      .getSettingsEssentials()
      .catch((err) => {
        console.error('Failed to fetch settings essentials:', err);
        return null;
      })
      .then((res) => res?.data);

    if (
      // Tries to access any route but setup is not completed
      appInfo &&
      !appInfo.initial_setup_completed &&
      !location.pathname.startsWith('/setup-wizard')
    ) {
      useSetupWizardStore.getState().reset();
      throw redirect({ to: '/setup-wizard', replace: true });
    } else if (
      // Tries to access setup wizard but setup is already completed
      appInfo?.initial_setup_completed &&
      location.pathname.startsWith('/setup-wizard')
    ) {
      throw redirect({ to: '/vpn-overview', replace: true });
    }

    // only auto check for auth state if route is not in /auth flow
    if (location.pathname.startsWith('/auth')) {
      return;
    }

    if (!isPresent(useAuth.getState().user)) {
      try {
        const { data: user } = await api.user.getMe().catch(() => {
          throw redirect({ to: '/auth', replace: true });
        });

        if (!user.id) {
          throw redirect({ to: '/auth', replace: true });
        }

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
  return (
    <SnackbarManager>
      <Outlet />
    </SnackbarManager>
  );
}
