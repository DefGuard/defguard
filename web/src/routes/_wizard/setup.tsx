import type { QueryClient } from '@tanstack/react-query';
import { createFileRoute, redirect } from '@tanstack/react-router';
import type { AxiosError } from 'axios';
import { SetupPage } from '../../pages/SetupPage/SetupPage';
import { SetupPageStep, type SetupPageStepValue } from '../../pages/SetupPage/types';
import { useSetupWizardStore } from '../../pages/SetupPage/useSetupWizardStore';
import api from '../../shared/api/api';
import type { InitialSetupStepValue } from '../../shared/api/types';
import {
  getSessionInfoQueryOptions,
  getSettingsEssentialsQueryOptions,
} from '../../shared/query';

const requiresSetupAuth = (step: InitialSetupStepValue) =>
  step !== 'Welcome' && step !== 'AdminUser';

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

const handleWizardRedirect = async ({ client }: { client: QueryClient }) => {
  const sessionInfo = (await client.ensureQueryData(getSessionInfoQueryOptions)).data;
  const settingsEssentials = (
    await client.ensureQueryData(getSettingsEssentialsQueryOptions)
  ).data;

  if (sessionInfo.wizard_flags?.initial_wizard_completed) {
    throw redirect({
      to: '/auth/login',
      replace: true,
    });
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
  beforeLoad: async ({ context }) => {
    await handleWizardRedirect({
      client: context.queryClient,
    });
  },
  component: SetupPage,
});
