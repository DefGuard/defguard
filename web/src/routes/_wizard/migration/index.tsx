import { createFileRoute, redirect } from '@tanstack/react-router';
import { AppLoaderPage } from '../../../pages/AppLoaderPage/AppLoaderPage';
import { MigrationWizardPage } from '../../../pages/MigrationWizardPage/MigrationWizardPage';
import { useMigrationWizardStore } from '../../../pages/MigrationWizardPage/store/useMigrationWizardStore';
import { ActiveWizard } from '../../../shared/api/types';
import {
  getMigrationStateQueryOptions,
  getSessionInfoQueryOptions,
  getSettingsQueryOptions,
} from '../../../shared/query';

export const Route = createFileRoute('/_wizard/migration/')({
  component: MigrationWizardPage,
  pendingComponent: AppLoaderPage,
  beforeLoad: async ({ context }) => {
    const sessionInfo = (await context.queryClient.fetchQuery(getSessionInfoQueryOptions))
      .data;
    if (
      !sessionInfo.authorized ||
      !sessionInfo.active_wizard ||
      (sessionInfo.active_wizard && sessionInfo.active_wizard !== ActiveWizard.Migration)
    ) {
      throw redirect({
        to: '/auth',
        replace: true,
      });
    }
    const migrationState = (
      await context.queryClient.fetchQuery(getMigrationStateQueryOptions)
    ).data;
    if (migrationState?.location_state !== null) {
      throw redirect({ to: '/migration/locations', replace: true });
    }
    useMigrationWizardStore.setState(migrationState);
  },
  loader: async ({ context }) => {
    return Promise.all([
      context.queryClient.fetchQuery(getSessionInfoQueryOptions),
      context.queryClient.fetchQuery(getSettingsQueryOptions),
    ]);
  },
});
