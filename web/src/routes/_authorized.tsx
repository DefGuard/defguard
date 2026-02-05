import { createFileRoute, Outlet, redirect } from '@tanstack/react-router';
import { DisplayListModal } from '../shared/components/DisplayListModal/DisplayListModal';
import { SelectionModal } from '../shared/components/modals/SelectionModal/SelectionModal';
import { UpgradePlanModalManager } from '../shared/components/modals/UpgradePlanModalManager/UpgradePlanModalManager';
import { useAuth } from '../shared/hooks/useAuth';
import { AppConfigProvider } from '../shared/providers/AppConfigProvider';
import { AppUserProvider } from '../shared/providers/AppUserProvider';
import { getUserMeQueryOptions } from '../shared/query';

export const Route = createFileRoute('/_authorized')({
  component: RouteComponent,
  beforeLoad: async ({ context }) => {
    if (!useAuth.getState().user) {
      try {
        const user = (
          await (
            await context.queryClient.ensureQueryData(getUserMeQueryOptions)
          )()
        ).data;

        // Invalid user object
        if (!user.id) {
          useAuth.getState().reset();
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
