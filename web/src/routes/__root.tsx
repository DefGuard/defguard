import type { QueryClient } from '@tanstack/react-query';
import { createRootRouteWithContext, Outlet } from '@tanstack/react-router';
import { AppLoaderPage } from '../pages/AppLoaderPage/AppLoaderPage';
import { SnackbarManager } from '../shared/defguard-ui/providers/snackbar/SnackbarManager';

interface RouterContext {
  queryClient: QueryClient;
}

export const Route = createRootRouteWithContext<RouterContext>()({
  component: RootComponent,
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
