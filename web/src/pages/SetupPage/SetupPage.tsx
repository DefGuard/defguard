import { useNavigate } from '@tanstack/react-router';
import { type ReactNode, useEffect, useMemo } from 'react';
import { m } from '../../paraglide/messages';
import { Controls } from '../../shared/components/Controls/Controls';
import type { WizardPageStep } from '../../shared/components/wizard/types';
import { WizardPage } from '../../shared/components/wizard/WizardPage/WizardPage';
import { Button } from '../../shared/defguard-ui/components/Button/Button';
import { SizedBox } from '../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../shared/defguard-ui/types';
import { isPresent } from '../../shared/defguard-ui/utils/isPresent';
import { useApp } from '../../shared/hooks/useApp';
import worldMap from './assets/world-map.png';
import { SetupAdminUserStep } from './steps/SetupAdminUserStep';
import { SetupCertificateAuthorityStep } from './steps/SetupCertificateAuthorityStep';
import { SetupCertificateAuthoritySummaryStep } from './steps/SetupCertificateAuthoritySummaryStep';
import { SetupConfirmationStep } from './steps/SetupConfirmationStep';
import { SetupEdgeAdoptionStep } from './steps/SetupEdgeAdoptionStep';
import { SetupEdgeComponentStep } from './steps/SetupEdgeComponentStep';
import { SetupGeneralConfigStep } from './steps/SetupGeneralConfigStep';
import { SetupPageStep, type SetupPageStepValue } from './types';
import { useSetupWizardStore } from './useSetupWizardStore';

export const SetupPage = () => {
  const activeStep = useSetupWizardStore((s) => s.activeStep);
  const settingsEssentials = useApp((s) => s.settingsEssentials);
  const isOnWelcomePage = useSetupWizardStore((s) => s.isOnWelcomePage);
  const navigate = useNavigate();

  const stepsConfig = useMemo(
    (): Record<SetupPageStepValue, WizardPageStep> => ({
      adminUser: {
        id: SetupPageStep.AdminUser,
        order: 1,
        label: m.initial_setup_step_admin_user_label(),
        description: m.initial_setup_step_admin_user_description(),
      },
      generalConfig: {
        id: SetupPageStep.GeneralConfig,
        order: 2,
        label: m.initial_setup_step_general_config_label(),
        description: m.initial_setup_step_general_config_description(),
      },
      certificateAuthority: {
        id: SetupPageStep.CertificateAuthority,
        order: 3,
        label: m.initial_setup_step_certificate_authority_label(),
        description: m.initial_setup_step_certificate_authority_description(),
      },
      certificateAuthoritySummary: {
        id: SetupPageStep.CASummary,
        order: 4,
        label: m.initial_setup_step_certificate_authority_summary_label(),
        description: m.initial_setup_step_certificate_authority_summary_description(),
      },
      edgeComponent: {
        id: SetupPageStep.EdgeComponent,
        order: 5,
        label: m.initial_setup_step_edge_component_label(),
        description: m.initial_setup_step_edge_component_description(),
      },
      edgeAdoption: {
        id: SetupPageStep.EdgeAdoption,
        order: 6,
        label: m.initial_setup_step_edge_adoption_label(),
        description: m.initial_setup_step_edge_adoption_description(),
      },
      confirmation: {
        id: SetupPageStep.Confirmation,
        order: 7,
        label: m.initial_setup_step_confirmation_label(),
        description: m.initial_setup_step_confirmation_description(),
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
      edgeAdoption: <SetupEdgeAdoptionStep />,
      confirmation: <SetupConfirmationStep />,
    }),
    [],
  );

  const handleStartWizard = () => {
    useSetupWizardStore.getState().setActiveStep(SetupPageStep.AdminUser);
    useSetupWizardStore.setState({ isOnWelcomePage: false });
  };

  const WelcomePageContent = () => (
    <div className="left">
      <SizedBox height={ThemeSpacing.Xl} />
      <Controls>
        <Button
          text={m.initial_setup_welcome_button_configure()}
          onClick={handleStartWizard}
        />
      </Controls>
    </div>
  );

  useEffect(() => {
    if (isPresent(settingsEssentials) && settingsEssentials.initial_setup_completed) {
      navigate({ to: '/vpn-overview', replace: true });
    }
  }, [settingsEssentials?.initial_setup_completed, navigate, settingsEssentials]);

  return (
    <WizardPage
      activeStep={activeStep}
      subtitle={m.initial_setup_wizard_subtitle()}
      title={m.initial_setup_wizard_title()}
      steps={stepsConfig}
      id="setup-wizard"
      isOnWelcomePage={isOnWelcomePage}
      welcomePageConfig={{
        title: m.initial_setup_welcome_title(),
        subtitle: m.initial_setup_welcome_subtitle(),
        content: <WelcomePageContent />,
        media: <img src={worldMap} />,
      }}
    >
      {stepsComponents[activeStep]}
    </WizardPage>
  );
};
