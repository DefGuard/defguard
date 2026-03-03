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

/// Whether the current wizard step requires an authenticated setup session.
/// Both initial and auto-adoption wizards allow unauthenticated access
/// only on `welcome` and `admin_user` steps.
const requiresSetupAuth = (step: string) => step !== 'welcome' && step !== 'admin_user';

const handleWizardRedirect = async ({
  location,
  client,
}: {
  location: ParsedLocation;
  client: QueryClient;
}): Promise<void> => {
  // Always fetch fresh settings from the backend to get the current wizard state,
  // bypassing any stale cached data.
  const settingsEssentials = (await client.fetchQuery(getSettingsEssentialsQueryOptions))
    .data;
  useApp.setState({
    settingsEssentials,
  });

  const applyWizardStepFromServer = (step: InitialSetupStepValue) => {
    const stepMap: Record<InitialSetupStepValue, SetupPageStepValue> = {
      welcome: SetupPageStep.AdminUser,
      admin_user: SetupPageStep.AdminUser,
      general_configuration: SetupPageStep.GeneralConfig,
      ca: SetupPageStep.CertificateAuthority,
      ca_summary: SetupPageStep.CASummary,
      edge_component: SetupPageStep.EdgeComponent,
      confirmation: SetupPageStep.Confirmation,
      finished: SetupPageStep.Confirmation,
    };

    useSetupWizardStore.setState({
      activeStep: stepMap[step],
      isOnWelcomePage: step === 'welcome',
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

    const isAutoAdoptionFlowStarted = step !== 'welcome';

    useAutoAdoptionSetupWizardStore.setState({
      activeStep: stepMap[step],
      isAutoAdoptionFlowStarted,
    });
  };

  // Setup already completed — redirect away from wizard.
  if (
    settingsEssentials.initial_setup_completed &&
    location.pathname.startsWith('/setup')
  ) {
    throw redirect({ to: '/auth/login', replace: true });
  }

  // Determine the current wizard step based on active wizard type.
  let currentStep: string = settingsEssentials.initial_setup_step;
  if (settingsEssentials.active_wizard === 'auto_adoption') {
    try {
      const autoAdoptionStatus = await api.initial_setup.getAutoAdoptionResult();
      currentStep = autoAdoptionStatus.data.step;
    } catch {
      // If we can't fetch auto-adoption status, default to welcome (no auth needed).
      currentStep = 'welcome';
    }
  }

  // Check if the current step requires setup authentication.
  if (requiresSetupAuth(currentStep)) {
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

  // Use backend-provided active_wizard to decide which wizard flow to show.
  if (settingsEssentials.active_wizard === 'auto_adoption') {
    useSetupWizardStore.getState().setAutoAdoptionPath(true);
    applyAutoAdoptionStepFromServer(currentStep as AutoAdoptionAdoptionStepValue);
  } else {
    useSetupWizardStore.getState().startInitialWizardFlow();
    applyWizardStepFromServer(settingsEssentials.initial_setup_step);
  }
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
