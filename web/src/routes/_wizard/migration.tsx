import { createFileRoute } from '@tanstack/react-router';
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
  loader: async ({ context }) => {
    return Promise.all([
      context.queryClient.ensureQueryData(getSessionInfoQueryOptions),
      context.queryClient.ensureQueryData(getSettingsQueryOptions),
      context.queryClient.ensureQueryData(getMigrationStateQueryOptions),
    ]);
  },
});
