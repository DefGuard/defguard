import { useNavigate } from '@tanstack/react-router';
import { type ReactNode, useEffect, useMemo } from 'react';
import { Controls } from '../../shared/components/Controls/Controls';
import type { WizardPageStep } from '../../shared/components/wizard/types';
import { WizardPage } from '../../shared/components/wizard/WizardPage/WizardPage';
import { Button } from '../../shared/defguard-ui/components/Button/Button';
import { SizedBox } from '../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../shared/defguard-ui/types';
import { useApp } from '../../shared/hooks/useApp';
import worldMap from './assets/world-map.png';
import { SetupAdminUserStep } from './steps/SetupAdminUserStep';
import { SetupCertificateAuthorityStep } from './steps/SetupCertificateAuthorityStep';
import { SetupCertificateAuthoritySummaryStep } from './steps/SetupCertificateAuthoritySummaryStep';
import { SetupConfirmationStep } from './steps/SetupConfirmationStep';
import { SetupEdgeAdaptationStep } from './steps/SetupEdgeAdaptationStep';
import { SetupEdgeComponentStep } from './steps/SetupEdgeComponentStep';
import { SetupGeneralConfigStep } from './steps/SetupGeneralConfigStep';
import { SetupPageStep, type SetupPageStepValue } from './types';
import { useSetupWizardStore } from './useSetupWizardStore';

export const SetupPage = () => {
  const activeStep = useSetupWizardStore((s) => s.activeStep);
  const settingsEssentials = useApp((s) => s.settingsEssentials);
  const showWelcome = useSetupWizardStore((s) => s.showWelcome);
  const navigate = useNavigate();

  const stepsConfig = useMemo(
    (): Record<SetupPageStepValue, WizardPageStep> => ({
      adminUser: {
        id: SetupPageStep.AdminUser,
        order: 1,
        label: 'Create Admin User',
        description:
          'Manage core details and connection parameters for your VPN location.',
      },
      generalConfig: {
        id: SetupPageStep.GeneralConfig,
        order: 2,
        label: 'General Configuration',
        description:
          'Manage core details and connection parameters for your VPN location.',
      },
      certificateAuthority: {
        id: SetupPageStep.CertificateAuthority,
        order: 3,
        label: 'Certificate Authority',
        description: 'Securing component communication',
      },
      certificateAuthoritySummary: {
        id: SetupPageStep.CASummary,
        order: 4,
        label: 'Certificate Authority Summary',
        description: 'Securing component communication',
      },
      edgeComponent: {
        id: SetupPageStep.EdgeComponent,
        order: 5,
        label: 'Edge Component',
        description:
          'Set up your VPN proxy quickly and ensure secure, optimized traffic flow for your users.',
      },
      edgeAdaptation: {
        id: SetupPageStep.EdgeAdaptation,
        order: 6,
        label: 'Edge Component Adaptation',
        description:
          'Review the system’s checks and see if any issues need attention before deployment.',
      },
      confirmation: {
        id: SetupPageStep.Confirmation,
        order: 7,
        label: 'Confirmation',
        description: 'Your configuration was successful. You’re all set.',
      },
    }),
    [],
  );

  const stepsComponents = useMemo(
    (): Record<SetupPageStepValue, ReactNode> => ({
      adminUser: <SetupAdminUserStep />,
      generalConfig: <SetupGeneralConfigStep />,
      certificateAuthority: <SetupCertificateAuthorityStep />,
      certificateAuthoritySummary: <SetupCertificateAuthoritySummaryStep />,
      edgeComponent: <SetupEdgeComponentStep />,
      edgeAdaptation: <SetupEdgeAdaptationStep />,
      confirmation: <SetupConfirmationStep />,
    }),
    [],
  );

  const handleStartWizard = () => {
    useSetupWizardStore.getState().setActiveStep(SetupPageStep.AdminUser);
    useSetupWizardStore.setState({ showWelcome: false });
  };

  const WelcomePageContent = () => (
    <div className="left">
      <SizedBox height={ThemeSpacing.Xl} />
      <Controls>
        <Button text={'Configure Defguard'} onClick={handleStartWizard} />
      </Controls>
    </div>
  );

  useEffect(() => {
    if (settingsEssentials.initial_setup_completed) {
      navigate({ to: '/vpn-overview', replace: true });
    }
  }, [settingsEssentials.initial_setup_completed, navigate]);

  return (
    <WizardPage
      activeStep={activeStep}
      onClose={() => {}}
      subtitle="This wizard will guide you through the initial configuration of your Defguard instance."
      title="Initial Setup Wizard"
      steps={stepsConfig}
      id="setup-wizard"
      showWelcome={showWelcome}
      welcomePageConfig={{
        title: 'Welcome to Defguard initial configuration wizard.',
        subtitle:
          'This wizard walks you through the steps to configure your VPN connection with a simple and intuitive setup process.',
        content: <WelcomePageContent />,
        media: <img src={worldMap} />,
      }}
    >
      {stepsComponents[activeStep]}
    </WizardPage>
  );
};
