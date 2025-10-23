import type { QueryClient } from '@tanstack/react-query';
import { createRootRouteWithContext, Outlet } from '@tanstack/react-router';
import { AppAuthProvider } from '../shared/providers/AppAuthProvider';
import { AppConfigProvider } from '../shared/providers/AppConfigProvider';

interface RouterContext {
  queryClient: QueryClient;
}

export const Route = createRootRouteWithContext<RouterContext>()({
  component: RootComponent,
});

function RootComponent() {
  return (
    <AppConfigProvider>
      <AppAuthProvider>
        <Outlet />
      </AppAuthProvider>
    </AppConfigProvider>
  );
}
