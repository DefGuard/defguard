import './style.scss';
import { useNavigate } from '@tanstack/react-router';
import { type ReactNode, useCallback, useMemo } from 'react';
import { m } from '../../paraglide/messages';
import type { WizardPageStep } from '../../shared/components/wizard/types';
import { WizardCover } from '../../shared/components/wizard/WizardCoverImage/types';
import { WizardCoverImage } from '../../shared/components/wizard/WizardCoverImage/WizardCoverImage';
import { WizardPage } from '../../shared/components/wizard/WizardPage/WizardPage';
import { Button } from '../../shared/defguard-ui/components/Button/Button';
import { Divider } from '../../shared/defguard-ui/components/Divider/Divider';
import { SizedBox } from '../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../shared/defguard-ui/types';
import { SetupConfirmationStep } from './steps/SetupConfirmationStep';
import { SetupDeployGatewayStep } from './steps/SetupDeployGatewayStep';
import { SetupGatewayAdoptionStep } from './steps/SetupGatewayAdaptationStep';
import { SetupGatewayComponentStep } from './steps/SetupGatewayComponentStep';
import { GatewaySetupStep, type GatewaySetupStepValue } from './types';
import { useGatewayWizardStore } from './useGatewayWizardStore';

export const GatewaySetupPage = () => {
  const activeStep = useGatewayWizardStore((s) => s.activeStep);
  const isOnWelcomePage = useGatewayWizardStore((s) => s.isOnWelcomePage);
  const setIsOnWelcomePage = useGatewayWizardStore((s) => s.setisOnWelcomePage);
  const isMigrationWizard = useGatewayWizardStore((s) => s.isMigrationWizard);
  const navigate = useNavigate();

  const onClose = useCallback(() => {
    if (isMigrationWizard) {
      navigate({ to: '/migration/locations', replace: true });
      return;
    }
    navigate({ to: '/locations', replace: true }).then(() => {
      setTimeout(() => {
        useGatewayWizardStore.getState().reset();
      }, 100);
    });
  }, [isMigrationWizard, navigate]);

  const stepsConfig = useMemo(
    (): Record<GatewaySetupStepValue, WizardPageStep> => ({
      deployGateway: {
        order: 1,
        id: GatewaySetupStep.DeployGateway,
        label: m.gateway_setup_step_deploy_label(),
        description: m.gateway_setup_step_deploy_description(),
      },
      gatewayComponent: {
        id: GatewaySetupStep.GatewayComponent,
        order: 2,
        label: m.gateway_setup_step_gateway_component_label(),
        description: m.gateway_setup_step_gateway_component_description(),
      },
      gatewayAdoption: {
        id: GatewaySetupStep.GatewayAdoption,
        order: 3,
        label: m.gateway_setup_step_gateway_adoption_label(),
        description: m.gateway_setup_step_gateway_adoption_description(),
      },
      confirmation: {
        id: GatewaySetupStep.Confirmation,
        order: 4,
        label: m.gateway_setup_step_confirmation_label(),
        description: m.gateway_setup_step_confirmation_description(),
      },
    }),
    [],
  );

  const stepsComponents = useMemo(
    (): Record<GatewaySetupStepValue, ReactNode> => ({
      deployGateway: <SetupDeployGatewayStep />,
      gatewayComponent: <SetupGatewayComponentStep />,
      gatewayAdoption: <SetupGatewayAdoptionStep />,
      confirmation: <SetupConfirmationStep />,
    }),
    [],
  );

  const WelcomePageContent = () => (
    <>
      <SizedBox height={ThemeSpacing.Xl2} />
      <Divider />
      <SizedBox height={ThemeSpacing.Xl2} />
      <Button
        text={m.gateway_setup_controls_configure()}
        onClick={() => setIsOnWelcomePage(false)}
      />
    </>
  );

  return (
    <WizardPage
      activeStep={activeStep}
      onClose={onClose}
      subtitle={m.gateway_setup_page_subtitle()}
      title={m.gateway_setup_page_title()}
      steps={stepsConfig}
      id="gw-wizard"
      isOnWelcomePage={isOnWelcomePage}
      welcomePageConfig={{
        title: m.gateway_setup_welcome_title(),
        subtitle: m.gateway_setup_welcome_subtitle(),
        content: <WelcomePageContent />,
        docsLink: 'https://docs.defguard.net/edge-component/deployment',
        docsText: m.gateway_setup_welcome_docs_text(),
        media: <WizardCoverImage variant={WizardCover.Gateway} />,
        onClose,
      }}
    >
      {stepsComponents[activeStep]}
    </WizardPage>
  );
};
