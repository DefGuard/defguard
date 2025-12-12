import type { QueryClient } from '@tanstack/react-query';
import { createRootRouteWithContext, Outlet } from '@tanstack/react-router';
import { AppAuthProvider } from '../shared/providers/AppAuthProvider';

interface RouterContext {
  queryClient: QueryClient;
}

export const Route = createRootRouteWithContext<RouterContext>()({
  component: RootComponent,
});

function RootComponent() {
  return (
    <AppAuthProvider>
      <Outlet />
    </AppAuthProvider>
  );
}
