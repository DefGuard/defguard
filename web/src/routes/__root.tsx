import type { QueryClient } from '@tanstack/react-query';
import {
  createRootRouteWithContext,
  Outlet,
  type ParsedLocation,
  redirect,
} from '@tanstack/react-router';
import { AppLoaderPage } from '../pages/AppLoaderPage/AppLoaderPage';
import { useSetupWizardStore } from '../pages/SetupPage/useSetupWizardStore';
import { SnackbarManager } from '../shared/defguard-ui/providers/snackbar/SnackbarManager';
import { useApp } from '../shared/hooks/useApp';
import { useAuth } from '../shared/hooks/useAuth';
import { getUserMeQueryOptions } from '../shared/query';

interface RouterContext {
  queryClient: QueryClient;
}

// Handles the initial wizard redirect.
// All routes should redirect to the setup wizard if the initial setup is not completed.
const handleWizardRedirect = async (location: ParsedLocation) => {
  const settingsEssentials = useApp((s) => s.settingsEssentials);

  // Tries to access any route but setup is not completed
  const setupNotCompletedAnyAccess =
    !settingsEssentials.initial_setup_completed &&
    !location.pathname.startsWith('/setup-wizard');

  // Tries to access setup wizard but setup is already completed
  const setupCompletedButAccessingWizard =
    settingsEssentials.initial_setup_completed &&
    location.pathname.startsWith('/setup-wizard');

  if (setupNotCompletedAnyAccess) {
    useSetupWizardStore.getState().reset();
    throw redirect({ to: '/setup-wizard', replace: true });
  } else if (setupCompletedButAccessingWizard) {
    throw redirect({ to: '/vpn-overview', replace: true });
  }
};

export const Route = createRootRouteWithContext<RouterContext>()({
  component: RootComponent,
  beforeLoad: async ({ location, context }) => {
    await handleWizardRedirect(location);

    if (
      location.pathname.startsWith('/auth') ||
      location.pathname.startsWith('/setup-wizard')
    ) {
      return;
    }

    try {
      const user = (
        await (
          await context.queryClient.ensureQueryData(getUserMeQueryOptions)
        )()
      ).data;

      // Invalid user object
      if (!user.id) {
        throw redirect({ to: '/auth/login', replace: true });
      }

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
