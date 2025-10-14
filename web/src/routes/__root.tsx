import { createRootRoute, Outlet } from '@tanstack/react-router';
import { AppAuthProvider } from '../shared/providers/AppAuthProvider';

export const Route = createRootRoute({
  component: RootComponent,
});

function RootComponent() {
  return (
    <AppAuthProvider>
      <Outlet />
    </AppAuthProvider>
  );
}
