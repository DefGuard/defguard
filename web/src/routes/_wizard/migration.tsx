import { createFileRoute } from '@tanstack/react-router';
import { AppLoaderPage } from '../../pages/AppLoaderPage/AppLoaderPage';
import { MigrationWizardPage } from '../../pages/MigrationWizardPage/MigrationWizardPage';
import { getSessionInfoQueryOptions, getSettingsQueryOptions } from '../../shared/query';

export const Route = createFileRoute('/_wizard/migration')({
  component: MigrationWizardPage,
  pendingComponent: AppLoaderPage,
  loader: async ({ context }) => {
    await context.queryClient.ensureQueryData(getSessionInfoQueryOptions);
    return context.queryClient.ensureQueryData(getSettingsQueryOptions);
  },
});
