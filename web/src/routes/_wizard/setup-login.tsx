import { createFileRoute, redirect } from '@tanstack/react-router';
import type { AxiosError } from 'axios';
import { SetupLoginPage } from '../../pages/SetupPage/SetupLoginPage';
import api from '../../shared/api/api';
import { useApp } from '../../shared/hooks/useApp';
import { getSettingsEssentialsQueryOptions } from '../../shared/query';

const requiresSetupAuth = (step: string) => step !== 'welcome' && step !== 'admin_user';

const hasSetupSession = async (): Promise<boolean> => {
  try {
    await api.initial_setup.session();
    return true;
  } catch (error) {
    const status = (error as AxiosError).response?.status;
    if (status === 401 || status === 403) {
      return false;
    }
    throw error;
  }
};

export const Route = createFileRoute('/_wizard/setup-login')({
  beforeLoad: async ({ context }) => {
    const settingsEssentials = (
      await context.queryClient.ensureQueryData(getSettingsEssentialsQueryOptions)
    ).data;
    useApp.setState({ settingsEssentials });

    if (settingsEssentials.initial_setup_completed) {
      throw redirect({ to: '/auth/login', replace: true });
    }

    const { data: wizardState } = await api.initial_setup.getWizardState();
    useApp.setState({ wizardState });

    const currentStep: string =
      wizardState.active_wizard === 'auto_adoption'
        ? (wizardState.auto_adoption_state?.step ?? 'welcome')
        : (wizardState.initial_setup_state?.step ?? 'welcome');

    if (!requiresSetupAuth(currentStep)) {
      throw redirect({ to: '/setup', replace: true });
    }

    const sessionActive = await hasSetupSession();
    if (sessionActive) {
      throw redirect({ to: '/setup', replace: true });
    }
  },
  component: SetupLoginPage,
});
