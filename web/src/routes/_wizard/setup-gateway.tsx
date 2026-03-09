import { createFileRoute, redirect } from '@tanstack/react-router';
import { GatewaySetupPage } from '../../pages/GatewaySetupPage/GatewaySetupPage';
import { useGatewayWizardStore } from '../../pages/GatewaySetupPage/useGatewayWizardStore';
import { MigrationWizardStep } from '../../pages/MigrationWizardPage/types';
import { ActiveWizard } from '../../shared/api/types';
import {
  getMigrationStateQueryOptions,
  getSessionInfoQueryOptions,
} from '../../shared/query';

export const Route = createFileRoute('/_wizard/setup-gateway')({
  component: GatewaySetupPage,
  beforeLoad: async ({ context }) => {
    const sessionInfo = (await context.queryClient.fetchQuery(getSessionInfoQueryOptions))
      .data;
    if (!sessionInfo.isAdmin) {
      throw redirect({ to: '/auth', replace: true });
    }
    if (sessionInfo.active_wizard === ActiveWizard.Migration) {
      const migrationState = (
        await context.queryClient.fetchQuery(getMigrationStateQueryOptions)
      ).data;
      if (
        !migrationState ||
        migrationState.current_step !== MigrationWizardStep.Confirmation ||
        useGatewayWizardStore.getState().network_id === null
      ) {
        throw redirect({ to: '/migration', replace: true });
      }
    }
    if (useGatewayWizardStore.getState().network_id === null) {
      throw redirect({ to: '/locations', replace: true });
    }
  },
});
