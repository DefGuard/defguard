import { createFileRoute, Outlet, redirect } from '@tanstack/react-router';
import { DisplayListModal } from '../shared/components/DisplayListModal/DisplayListModal';
import { SelectionModal } from '../shared/components/modals/SelectionModal/SelectionModal';
import { UpgradePlanModalManager } from '../shared/components/modals/UpgradePlanModalManager/UpgradePlanModalManager';
import { useAuth } from '../shared/hooks/useAuth';
import { AppConfigProvider } from '../shared/providers/AppConfigProvider';
import { AppUserProvider } from '../shared/providers/AppUserProvider';

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
    <AppUserProvider>
      <AppConfigProvider>
        <Outlet />
        <DisplayListModal />
        <SelectionModal />
        <UpgradePlanModalManager />
      </AppConfigProvider>
    </AppUserProvider>
  );
}
