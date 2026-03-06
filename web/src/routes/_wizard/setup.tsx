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
import {
  getSessionInfoQueryOptions,
  getSettingsEssentialsQueryOptions,
} from '../../shared/query';

/// Whether the current wizard step requires an authenticated setup session.
/// Both initial and auto-adoption wizards allow unauthenticated access
/// only on `welcome` and `admin_user` steps.
const requiresSetupAuth = (step: string) => step !== 'welcome' && step !== 'admin_user';

/// Maps a backend initial-wizard step to the frontend step enum.
const initialStepMap: Record<InitialSetupStepValue, SetupPageStepValue> = {
  welcome: SetupPageStep.AdminUser,
  admin_user: SetupPageStep.AdminUser,
  general_configuration: SetupPageStep.GeneralConfig,
  ca: SetupPageStep.CertificateAuthority,
  ca_summary: SetupPageStep.CASummary,
  edge_component: SetupPageStep.EdgeComponent,
  confirmation: SetupPageStep.Confirmation,
  finished: SetupPageStep.Confirmation,
};

/// Maps a backend auto-adoption step to the frontend step enum.
const autoAdoptionStepMap: Record<
  AutoAdoptionAdoptionStepValue,
  AutoAdoptionSetupStepValue
> = {
  welcome: AutoAdoptionSetupStep.AdminUser,
  admin_user: AutoAdoptionSetupStep.AdminUser,
  url_settings: AutoAdoptionSetupStep.UrlSettings,
  vpn_settings: AutoAdoptionSetupStep.VpnSettings,
  mfa_settings: AutoAdoptionSetupStep.MfaSetup,
  summary: AutoAdoptionSetupStep.Summary,
  finished: AutoAdoptionSetupStep.Summary,
};

const handleWizardRedirect = async ({
  client,
}: {
  location: ParsedLocation;
  client: QueryClient;
}): Promise<void> => {
  const sessionInfo = (await client.fetchQuery(getSessionInfoQueryOptions)).data;
  const settingsEssentials = (await client.fetchQuery(getSettingsEssentialsQueryOptions))
    .data;
  useApp.setState({ settingsEssentials });

  if (sessionInfo.active_wizard === null) {
    throw redirect({ to: '/auth/login', replace: true });
  }

  const { data: wizardState } = await api.initial_setup.getWizardState();
  useApp.setState({ wizardState });

  const isAutoAdoption = wizardState.active_wizard === 'auto_adoption';
  const currentStep: string = isAutoAdoption
    ? (wizardState.auto_adoption_state?.step ?? 'welcome')
    : (wizardState.initial_setup_state?.step ?? 'welcome');

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

  // Apply the server-provided step to the appropriate wizard store.
  if (isAutoAdoption) {
    useAutoAdoptionSetupWizardStore.setState({
      activeStep: autoAdoptionStepMap[currentStep as AutoAdoptionAdoptionStepValue],
      isAutoAdoptionFlowStarted: currentStep !== 'welcome',
    });
  } else {
    const initialStep = (wizardState.initial_setup_state?.step ??
      'welcome') as InitialSetupStepValue;
    useSetupWizardStore.setState({
      activeStep: initialStepMap[initialStep],
      isOnWelcomePage: initialStep === 'welcome',
    });
  }
};

const SetupWizardRouter = () => {
  const activeWizard = useApp((s) => s.wizardState?.active_wizard);

  if (activeWizard === 'auto_adoption') {
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
