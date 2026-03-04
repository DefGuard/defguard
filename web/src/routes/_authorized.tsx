import { createFileRoute, Outlet, redirect } from '@tanstack/react-router';
import { DisplayListModal } from '../shared/components/DisplayListModal/DisplayListModal';
import { LicenseExpiredModal } from '../shared/components/modals/license/LicenseExpiredModal/LicenseExpiredModal';
import { LimitReachedModal } from '../shared/components/modals/license/LimitReachedModal/LimitReachedModal';
import { UpgradeBusinessModal } from '../shared/components/modals/license/UpgradeBusinessModal/UpgradeBusinessModal';
import { UpgradeEnterpriseModal } from '../shared/components/modals/license/UpgradeEnterpriseModal/UpgradeEnterpriseModal';
import { SelectionModal } from '../shared/components/modals/SelectionModal/SelectionModal';
import { AppInfoProvider } from '../shared/providers/AppInfoProvider';
import { AppUserProvider } from '../shared/providers/AppUserProvider';
import { getSessionInfoQueryOptions } from '../shared/query';

export const Route = createFileRoute('/_authorized')({
  component: RouteComponent,
  beforeLoad: async ({ context }) => {
    const sessionInfo = (await context.queryClient.fetchQuery(getSessionInfoQueryOptions))
      .data;
    if (!sessionInfo.authorized) {
      throw redirect({ to: '/auth/login', replace: true });
    }
    if (sessionInfo.wizard_flags) {
      if (sessionInfo.wizard_flags.initial_wizard_in_progress) {
        throw redirect({ to: '/setup', replace: true });
      }
      if (sessionInfo.wizard_flags.migration_wizard_in_progress) {
        throw redirect({ to: '/migration', replace: true });
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
