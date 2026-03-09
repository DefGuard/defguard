import { createFileRoute, redirect } from '@tanstack/react-router';
import { LocationsMigrationWizardPage } from '../../../pages/LocationsMigrationWizardPage/LocationsMigrationWizardPage';
import { useMigrationWizardStore } from '../../../pages/MigrationWizardPage/store/useMigrationWizardStore';
import { ActiveWizard } from '../../../shared/api/types';
import { isPresent } from '../../../shared/defguard-ui/utils/isPresent';
import {
  getMigrationStateQueryOptions,
  getSessionInfoQueryOptions,
} from '../../../shared/query';

export const Route = createFileRoute('/_wizard/migration/locations')({
  beforeLoad: async ({ context }) => {
    const sessionInfo = (await context.queryClient.fetchQuery(getSessionInfoQueryOptions))
      .data;
    if (
      sessionInfo.active_wizard !== ActiveWizard.Migration ||
      !(sessionInfo.authorized && sessionInfo.isAdmin)
    ) {
      throw redirect({ to: '/auth', replace: true });
    }
    const migrationState = (
      await context.queryClient.fetchQuery(getMigrationStateQueryOptions)
    ).data;
    if (!isPresent(migrationState?.location_state)) {
      throw redirect({ to: '/migration', replace: true });
    }
    useMigrationWizardStore.setState(migrationState);
  },
  component: LocationsMigrationWizardPage,
});
