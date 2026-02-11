import type { QueryClient } from '@tanstack/react-query';
import { createFileRoute, type ParsedLocation, redirect } from '@tanstack/react-router';
import type { AxiosError } from 'axios';
import { SetupPage } from '../../pages/SetupPage/SetupPage';
import { SetupPageStep, type SetupPageStepValue } from '../../pages/SetupPage/types';
import { useSetupWizardStore } from '../../pages/SetupPage/useSetupWizardStore';
import api from '../../shared/api/api';
import type { InitialSetupStepValue } from '../../shared/api/types';
import { isPresent } from '../../shared/defguard-ui/utils/isPresent';
import { useApp } from '../../shared/hooks/useApp';
import { getSettingsEssentialsQueryOptions } from '../../shared/query';

const requiresSetupAuth = (step: InitialSetupStepValue) =>
  step !== 'Welcome' && step !== 'AdminUser';

const handleWizardRedirect = async ({
  location,
  client,
}: {
  location: ParsedLocation;
  client: QueryClient;
}) => {
  let settingsEssentials = useApp.getState().settingsEssentials;
  if (!isPresent(settingsEssentials)) {
    settingsEssentials = (await client.ensureQueryData(getSettingsEssentialsQueryOptions))
      .data;
    useApp.setState({
      settingsEssentials,
    });
  }

  const applyWizardStepFromServer = (step: InitialSetupStepValue) => {
    const stepMap: Record<InitialSetupStepValue, SetupPageStepValue> = {
      Welcome: SetupPageStep.AdminUser,
      AdminUser: SetupPageStep.AdminUser,
      GeneralConfiguration: SetupPageStep.GeneralConfig,
      Ca: SetupPageStep.CertificateAuthority,
      CaSummary: SetupPageStep.CASummary,
      EdgeComponent: SetupPageStep.EdgeComponent,
      Confirmation: SetupPageStep.Confirmation,
      Finished: SetupPageStep.Confirmation,
    };

    useSetupWizardStore.setState({
      activeStep: stepMap[step],
      isOnWelcomePage: step === 'Welcome',
    });
  };

  // Tries to access setup wizard but setup is already completed
  const setupCompletedButAccessingWizard =
    settingsEssentials.initial_setup_completed && location.pathname.startsWith('/setup');

  if (setupCompletedButAccessingWizard) {
    throw redirect({ to: '/auth/login', replace: true });
  }

  if (requiresSetupAuth(settingsEssentials.initial_setup_step)) {
    try {
      await api.initial_setup.session();
    } catch (error) {
      const status = (error as AxiosError).response?.status;
      if (status === 401 || status === 403) {
        throw redirect({ to: '/setup-login', replace: true });
      }
      throw error;
    }
  }

  applyWizardStepFromServer(settingsEssentials.initial_setup_step);
};

export const Route = createFileRoute('/_wizard/setup')({
  beforeLoad: async ({ context, location }) => {
    await handleWizardRedirect({
      client: context.queryClient,
      location,
    });
  },
  component: SetupPage,
});
