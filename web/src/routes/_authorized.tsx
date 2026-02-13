import { createFileRoute, Outlet, redirect } from '@tanstack/react-router';
import { DisplayListModal } from '../shared/components/DisplayListModal/DisplayListModal';
import { LicenseExpiredModal } from '../shared/components/modals/license/LicenseExpiredModal/LicenseExpiredModal';
import { LimitReachedModal } from '../shared/components/modals/license/LimitReachedModal/LimitReachedModal';
import { UpgradeBusinessModal } from '../shared/components/modals/license/UpgradeBusinessModal/UpgradeBusinessModal';
import { UpgradeEnterpriseModal } from '../shared/components/modals/license/UpgradeEnterpriseModal/UpgradeEnterpriseModal';
import { SelectionModal } from '../shared/components/modals/SelectionModal/SelectionModal';
import { useAuth } from '../shared/hooks/useAuth';
import { AppInfoProvider } from '../shared/providers/AppInfoProvider';
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
      <AppInfoProvider>
        <Outlet />
        <LimitReachedModal />
        <UpgradeBusinessModal />
        <UpgradeEnterpriseModal />
        <LicenseExpiredModal />
        <DisplayListModal />
        <SelectionModal />
      </AppInfoProvider>
    </AppUserProvider>
  );
}
