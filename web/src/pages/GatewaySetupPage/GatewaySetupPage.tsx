import './style.scss';
import { useNavigate } from '@tanstack/react-router';
import { type ReactNode, useMemo } from 'react';
import { m } from '../../paraglide/messages';
import { Controls } from '../../shared/components/Controls/Controls';
import type { WizardPageStep } from '../../shared/components/wizard/types';
import { WizardPage } from '../../shared/components/wizard/WizardPage/WizardPage';
import { Button } from '../../shared/defguard-ui/components/Button/Button';
import { Divider } from '../../shared/defguard-ui/components/Divider/Divider';
import { ThemeSpacing } from '../../shared/defguard-ui/types';
import welcomeImage from './assets/welcome_image.svg';
import { SetupConfirmationStep } from './steps/SetupConfirmationStep';
import { SetupGatewayAdoptionStep } from './steps/SetupGatewayAdaptationStep';
import { SetupGatewayComponentStep } from './steps/SetupGatewayComponentStep';
import { GatewaySetupStep, type GatewaySetupStepValue } from './types';
import { useGatewayWizardStore } from './useGatewayWizardStore';

export const GatewaySetupPage = () => {
  const activeStep = useGatewayWizardStore((s) => s.activeStep);
  const isOnWelcomePage = useGatewayWizardStore((s) => s.isOnWelcomePage);
  const setisOnWelcomePage = useGatewayWizardStore((s) => s.setisOnWelcomePage);
  const navigate = useNavigate();

  const stepsConfig = useMemo(
    (): Record<GatewaySetupStepValue, WizardPageStep> => ({
      gatewayComponent: {
        id: GatewaySetupStep.GatewayComponent,
        order: 1,
        label: m.gateway_setup_step_gateway_component_label(),
        description: m.gateway_setup_step_gateway_component_description(),
      },
      gatewayAdoption: {
        id: GatewaySetupStep.GatewayAdoption,
        order: 2,
        label: m.gateway_setup_step_gateway_adoption_label(),
        description: m.gateway_setup_step_gateway_adoption_description(),
      },
      confirmation: {
        id: GatewaySetupStep.Confirmation,
        order: 3,
        label: m.gateway_setup_step_confirmation_label(),
        description: m.gateway_setup_step_confirmation_description(),
      },
    }),
    [],
  );

  const stepsComponents = useMemo(
    (): Record<GatewaySetupStepValue, ReactNode> => ({
      gatewayComponent: <SetupGatewayComponentStep />,
      gatewayAdoption: <SetupGatewayAdoptionStep />,
      confirmation: <SetupConfirmationStep />,
    }),
    [],
  );

  const WelcomePageContent = () => (
    <>
      <Divider spacing={ThemeSpacing.Xl} />
      <div className="left">
        <Controls>
          <Button
            text={m.gateway_setup_controls_configure()}
            onClick={() => setisOnWelcomePage(false)}
          />
        </Controls>
      </div>
    </>
  );

  return (
    <WizardPage
      activeStep={activeStep}
      onClose={() => {
        navigate({ to: '/locations', replace: true }).then(() => {
          setTimeout(() => {
            useGatewayWizardStore.getState().reset();
          }, 100);
        });
      }}
      subtitle={m.gateway_setup_page_subtitle()}
      title={m.gateway_setup_page_title()}
      steps={stepsConfig}
      id="setup-wizard"
      isOnWelcomePage={isOnWelcomePage}
      welcomePageConfig={{
        title: m.gateway_setup_welcome_title(),
        subtitle: m.gateway_setup_welcome_subtitle(),
        content: <WelcomePageContent />,
        docsLink: 'https://docs.defguard.net/edge-component/deployment',
        docsText: m.gateway_setup_welcome_docs_text(),
        media: <img src={welcomeImage} alt={m.gateway_setup_welcome_image_alt()} />,
        onClose: () => {
          navigate({ to: '/locations', replace: true }).then(() => {
            setTimeout(() => {
              useGatewayWizardStore.getState().reset();
            }, 100);
          });
        },
      }}
    >
      {stepsComponents[activeStep]}
    </WizardPage>
  );
};
