import { createFileRoute, Outlet, redirect } from '@tanstack/react-router';
import { DisplayListModal } from '../shared/components/DisplayListModal/DisplayListModal';
import { ConfirmActionModal } from '../shared/components/modals/ConfirmActionModal/ConfirmActionModal';
import { LicenseExpiredModal } from '../shared/components/modals/license/LicenseExpiredModal/LicenseExpiredModal';
import { LicenseLimitConflictModal } from '../shared/components/modals/license/LicenseLimitConflictModal/LicenseLimitConflictModal';
import { LimitReachedModal } from '../shared/components/modals/license/LimitReachedModal/LimitReachedModal';
import { UpgradeBusinessModal } from '../shared/components/modals/license/UpgradeBusinessModal/UpgradeBusinessModal';
import { UpgradeEnterpriseModal } from '../shared/components/modals/license/UpgradeEnterpriseModal/UpgradeEnterpriseModal';
import { SelectionModal } from '../shared/components/modals/SelectionModal/SelectionModal';
import { AppInfoProvider } from '../shared/providers/AppInfoProvider';
import { AppUserProvider } from '../shared/providers/AppUserProvider';
import { getSessionInfoQueryOptions, getUserMeQueryOptions } from '../shared/query';

export const Route = createFileRoute('/_authorized')({
  component: RouteComponent,
  beforeLoad: async ({ context }) => {
    const sessionInfo = (await context.queryClient.fetchQuery(getSessionInfoQueryOptions))
      .data;
    if (!sessionInfo.authorized) {
      throw redirect({ to: '/auth/login', replace: true });
    }
    if (sessionInfo.active_wizard) {
      switch (sessionInfo.active_wizard) {
        case 'initial':
        case 'auto_adoption':
          throw redirect({ to: '/setup', replace: true });
        case 'migration':
          throw redirect({ to: '/migration', replace: true });
      }
    }

    if (sessionInfo.is_admin) {
      return;
    }

    const me = (await context.queryClient.fetchQuery(getUserMeQueryOptions)).data;

    if (location.pathname !== `/user/${me.username}`) {
      throw redirect({
        to: '/user/$username',
        params: {
          username: me.username,
        },
        replace: true,
      });
    }
  },
});

function RouteComponent() {
  return (
    <AppUserProvider>
      <AppInfoProvider>
        <Outlet />
        <LimitReachedModal />
        <LicenseLimitConflictModal />
        <UpgradeBusinessModal />
        <UpgradeEnterpriseModal />
        <LicenseExpiredModal />
        <DisplayListModal />
        <SelectionModal />
        <ConfirmActionModal />
      </AppInfoProvider>
    </AppUserProvider>
  );
}
