import type { QueryClient } from '@tanstack/react-query';
import { createFileRoute, type ParsedLocation, redirect } from '@tanstack/react-router';
import type { AxiosError } from 'axios';
import { AutoAdoptionSetupPage } from '../../pages/SetupPage/autoAdoption/AutoAdoptionSetupPage';
import {
  AutoAdoptionSetupStep,
  type AutoAdoptionSetupStepValue,
} from '../../pages/SetupPage/autoAdoption/types';
import { useAutoAdoptionSetupWizardStore } from '../../pages/SetupPage/autoAdoption/useAutoAdoptionSetupWizardStore';
import { SetupPage } from '../../pages/SetupPage/initial/SetupPage';
import {
  SetupPageStep,
  type SetupPageStepValue,
} from '../../pages/SetupPage/initial/types';
import { useSetupWizardStore } from '../../pages/SetupPage/initial/useSetupWizardStore';
import api from '../../shared/api/api';
import type {
  AutoAdoptionAdoptionStepValue,
  InitialSetupStepValue,
} from '../../shared/api/types';
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
}): Promise<boolean> => {
  // Always fetch fresh settings from the backend to get the current wizard step,
  // bypassing any stale cached data.
  const settingsEssentials = (await client.fetchQuery(getSettingsEssentialsQueryOptions))
    .data;
  useApp.setState({
    settingsEssentials,
  });

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

  const applyAutoAdoptionStepFromServer = (step: AutoAdoptionAdoptionStepValue) => {
    const stepMap: Record<AutoAdoptionAdoptionStepValue, AutoAdoptionSetupStepValue> = {
      welcome: AutoAdoptionSetupStep.AdminUser,
      admin_user: AutoAdoptionSetupStep.AdminUser,
      url_settings: AutoAdoptionSetupStep.UrlSettings,
      vpn_settings: AutoAdoptionSetupStep.VpnSettings,
      mfa_settings: AutoAdoptionSetupStep.MfaSetup,
      summary: AutoAdoptionSetupStep.Summary,
      finished: AutoAdoptionSetupStep.Summary,
    };

    // The Auto-adoption flow has been started if the server step is past "welcome".
    const isAutoAdoptionFlowStarted = step !== 'welcome';

    useAutoAdoptionSetupWizardStore.setState({
      activeStep: stepMap[step],
      isAutoAdoptionFlowStarted,
    });
  };

  // Tries to access setup wizard but setup is already completed
  const setupCompletedButAccessingWizard =
    settingsEssentials.initial_setup_completed && location.pathname.startsWith('/setup');

  if (setupCompletedButAccessingWizard) {
    throw redirect({ to: '/auth/login', replace: true });
  }

  let isAutoAdoptionPath = false;
  let autoAdoptionStep: AutoAdoptionAdoptionStepValue | undefined;
  if (!settingsEssentials.initial_setup_completed) {
    try {
      const autoAdoptionStatus = await api.initial_setup.getAutoAdoptionResult();
      const adoptionResult = autoAdoptionStatus.data.adoption_result;
      isAutoAdoptionPath = Object.keys(adoptionResult ?? {}).length > 0;
      if (isAutoAdoptionPath) {
        autoAdoptionStep = autoAdoptionStatus.data.step;
      }
    } catch {
      // Ignore auto-adoption status fetch failures and use the regular setup flow.
    }
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

  if (isAutoAdoptionPath) {
    useSetupWizardStore.getState().setAutoAdoptionPath(true);
    if (autoAdoptionStep !== undefined) {
      applyAutoAdoptionStepFromServer(autoAdoptionStep);
    } else {
      useAutoAdoptionSetupWizardStore.getState().startFlow();
    }
  } else {
    useSetupWizardStore.getState().startInitialWizardFlow();
    applyWizardStepFromServer(settingsEssentials.initial_setup_step);
  }

  return isAutoAdoptionPath;
};

const SetupWizardRouter = () => {
  const isAutoAdoptionPath = useSetupWizardStore((s) => s.isAutoAdoptionPath);

  if (isAutoAdoptionPath) {
    return <AutoAdoptionSetupPage />;
  }

  return <SetupPage />;
};

export const Route = createFileRoute('/_wizard/setup')({
  beforeLoad: async ({ context, location }) => {
    await handleWizardRedirect({
      client: context.queryClient,
      location,
    });
  },
  component: SetupWizardRouter,
});
