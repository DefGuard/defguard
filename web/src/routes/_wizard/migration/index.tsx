import { createFileRoute, redirect } from '@tanstack/react-router';
import { AppLoaderPage } from '../../../pages/AppLoaderPage/AppLoaderPage';
import { MigrationWizardPage } from '../../../pages/MigrationWizardPage/MigrationWizardPage';
import { useMigrationWizardStore } from '../../../pages/MigrationWizardPage/store/useMigrationWizardStore';
import { ActiveWizard } from '../../../shared/api/types';
import { isPresent } from '../../../shared/defguard-ui/utils/isPresent';
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
    if (isPresent(migrationState)) {
      useMigrationWizardStore.setState(migrationState);
    }
  },
  loader: async ({ context }) => {
    return Promise.all([
      context.queryClient.fetchQuery(getSessionInfoQueryOptions),
      context.queryClient.fetchQuery(getSettingsQueryOptions),
    ]);
  },
});
