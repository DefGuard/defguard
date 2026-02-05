import type { QueryClient } from '@tanstack/react-query';
import { createFileRoute, type ParsedLocation, redirect } from '@tanstack/react-router';
import { SetupPage } from '../../pages/SetupPage/SetupPage';
import { useSetupWizardStore } from '../../pages/SetupPage/useSetupWizardStore';
import { getSettingsEssentialsQueryOptions } from '../../shared/query';

const handleWizardRedirect = async ({
  location,
  client,
}: {
  location: ParsedLocation;
  client: QueryClient;
}) => {
  const settingsEssentials = (
    await (
      await client.ensureQueryData(getSettingsEssentialsQueryOptions)
    )()
  ).data;

  // Tries to access any route but setup is not completed
  const setupNotCompletedAnyAccess =
    !settingsEssentials.initial_setup_completed &&
    !location.pathname.startsWith('/setup-wizard');

  // Tries to access setup wizard but setup is already completed
  const setupCompletedButAccessingWizard =
    settingsEssentials.initial_setup_completed &&
    location.pathname.startsWith('/setup-wizard');

  if (setupNotCompletedAnyAccess) {
    useSetupWizardStore.getState().reset();
    throw redirect({ to: '/setup-wizard', replace: true });
  } else if (setupCompletedButAccessingWizard) {
    throw redirect({ to: '/auth/login', replace: true });
  }
};

export const Route = createFileRoute('/_wizard/setup-wizard')({
  beforeLoad: async ({ context, location }) => {
    await handleWizardRedirect({
      client: context.queryClient,
      location,
    });
  },
  component: SetupPage,
});
