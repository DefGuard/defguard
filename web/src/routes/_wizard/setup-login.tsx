import { createFileRoute, redirect } from '@tanstack/react-router';
import type { AxiosError } from 'axios';
import { SetupLoginPage } from '../../pages/SetupPage/SetupLoginPage';
import api from '../../shared/api/api';
import type { InitialSetupStepValue } from '../../shared/api/types';
import { isPresent } from '../../shared/defguard-ui/utils/isPresent';
import { useApp } from '../../shared/hooks/useApp';
import { getSettingsEssentialsQueryOptions } from '../../shared/query';

const requiresSetupAuth = (step: InitialSetupStepValue) =>
  step !== 'Welcome' && step !== 'AdminUser';

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
    let settingsEssentials = useApp.getState().settingsEssentials;
    if (!isPresent(settingsEssentials)) {
      settingsEssentials = (
        await context.queryClient.ensureQueryData(getSettingsEssentialsQueryOptions)
      ).data;
      useApp.setState({
        settingsEssentials,
      });
    }

    if (settingsEssentials.initial_setup_completed) {
      throw redirect({ to: '/auth/login', replace: true });
    }

    if (!requiresSetupAuth(settingsEssentials.initial_setup_step)) {
      throw redirect({ to: '/setup', replace: true });
    }

    const sessionActive = await hasSetupSession();
    if (sessionActive) {
      throw redirect({ to: '/setup', replace: true });
    }
  },
  component: SetupLoginPage,
});
