import { createFileRoute, Outlet, redirect } from '@tanstack/react-router';
import { DisplayListModal } from '../shared/components/DisplayListModal/DisplayListModal';
import { SelectionModal } from '../shared/components/modals/SelectionModal/SelectionModal';
import { useAuth } from '../shared/hooks/useAuth';
import { AppConfigProvider } from '../shared/providers/AppConfigProvider';

export const Route = createFileRoute('/_authorized')({
  component: RouteComponent,
  beforeLoad: () => {
    if (!useAuth.getState().user) {
      console.error('No auth session in store.');
      throw redirect({ to: '/auth', replace: true });
    }
  },
});

function RouteComponent() {
  return (
    <AppConfigProvider>
      <Outlet />
      <DisplayListModal />
      <SelectionModal />
    </AppConfigProvider>
  );
}
