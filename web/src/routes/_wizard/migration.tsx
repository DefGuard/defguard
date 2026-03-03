import { createFileRoute, redirect } from '@tanstack/react-router';
import { AppLoaderPage } from '../../pages/AppLoaderPage/AppLoaderPage';
import { MigrationWizardPage } from '../../pages/MigrationWizardPage/MigrationWizardPage';
import {
  getMigrationStateQueryOptions,
  getSessionInfoQueryOptions,
  getSettingsQueryOptions,
} from '../../shared/query';

export const Route = createFileRoute('/_wizard/migration')({
  component: MigrationWizardPage,
  pendingComponent: AppLoaderPage,
  beforeLoad: async ({ context }) => {
    const sessionInfo = (await context.queryClient.fetchQuery(getSessionInfoQueryOptions))
      .data;
    if (!sessionInfo.authorized) {
      throw redirect({
        to: '/auth',
        replace: true,
      });
    }
    if (
      sessionInfo.wizard_flags &&
      !sessionInfo.wizard_flags.migration_wizard_in_progress
    ) {
      throw redirect({
        to: '/auth',
        replace: true,
      });
    }
  },
  loader: async ({ context }) => {
    return Promise.all([
      context.queryClient.fetchQuery(getSessionInfoQueryOptions),
      context.queryClient.fetchQuery(getSettingsQueryOptions),
      context.queryClient.fetchQuery(getMigrationStateQueryOptions),
    ]);
  },
});
