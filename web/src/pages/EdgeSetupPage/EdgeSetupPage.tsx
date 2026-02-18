import './style.scss';
import { useNavigate } from '@tanstack/react-router';
import { type ReactNode, useMemo } from 'react';
import { m } from '../../paraglide/messages';
import { ActionCard } from '../../shared/components/ActionCard/ActionCard';
import { Controls } from '../../shared/components/Controls/Controls';
import type { WizardPageStep } from '../../shared/components/wizard/types';
import { WizardPage } from '../../shared/components/wizard/WizardPage/WizardPage';
import { Button } from '../../shared/defguard-ui/components/Button/Button';
import { Divider } from '../../shared/defguard-ui/components/Divider/Divider';
import { SizedBox } from '../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../shared/defguard-ui/types';
import deployImage from './assets/deploy.svg';
import welcomeImage from './assets/welcome_image.svg';
import { SetupConfirmationStep } from './steps/SetupConfirmationStep';
import { SetupEdgeAdoptionStep } from './steps/SetupEdgeAdoptionStep';
import { SetupEdgeComponentStep } from './steps/SetupEdgeComponentStep';
import { SetupEdgeDeployStep } from './steps/SetupEdgeDeployStep';
import { EdgeSetupStep, type EdgeSetupStepValue } from './types';
import { useEdgeWizardStore } from './useEdgeWizardStore';

export const EdgeSetupPage = () => {
  const activeStep = useEdgeWizardStore((s) => s.activeStep);
  const isOnWelcomePage = useEdgeWizardStore((s) => s.isOnWelcomePage);
  const setisOnWelcomePage = useEdgeWizardStore((s) => s.setisOnWelcomePage);
  const navigate = useNavigate();

  const stepsConfig = useMemo(
    (): Record<EdgeSetupStepValue, WizardPageStep> => ({
      edgeDeploy: {
        id: EdgeSetupStep.EdgeDeploy,
        label: m.edge_setup_step_deploy_label(),
        description: m.edge_setup_step_deploy_description(),
        order: 1,
      },
      edgeComponent: {
        id: EdgeSetupStep.EdgeComponent,
        order: 2,
        label: m.edge_setup_step_edge_component_label(),
        description: m.edge_setup_step_edge_component_description(),
      },
      edgeAdoption: {
        id: EdgeSetupStep.EdgeAdoption,
        order: 3,
        label: m.edge_setup_step_edge_adoption_label(),
        description: m.edge_setup_step_edge_adoption_description(),
      },
      confirmation: {
        id: EdgeSetupStep.Confirmation,
        order: 4,
        label: m.edge_setup_step_confirmation_label(),
        description: m.edge_setup_step_confirmation_description(),
      },
    }),
    [],
  );

  const stepsComponents = useMemo(
    (): Record<EdgeSetupStepValue, ReactNode> => ({
      edgeDeploy: <SetupEdgeDeployStep />,
      edgeComponent: <SetupEdgeComponentStep />,
      edgeAdoption: <SetupEdgeAdoptionStep />,
      confirmation: <SetupConfirmationStep />,
    }),
    [],
  );

  const WelcomePageContent = () => (
    <>
      <Divider spacing={ThemeSpacing.Xl} />
      <div className="left">
        <ActionCard
          title={m.edge_setup_welcome_deploy_title()}
          subtitle={m.edge_setup_welcome_deploy_subtitle()}
          imageSrc={deployImage}
        />
        <SizedBox height={ThemeSpacing.Xl} />
        <Controls>
          <Button
            text={m.edge_setup_controls_configure()}
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
        navigate({ to: '/vpn-overview', replace: true }).then(() => {
          setTimeout(() => {
            useEdgeWizardStore.getState().reset();
          }, 100);
        });
      }}
      subtitle={m.edge_setup_page_subtitle()}
      title={m.edge_setup_page_title()}
      steps={stepsConfig}
      id="setup-wizard"
      isOnWelcomePage={isOnWelcomePage}
      welcomePageConfig={{
        title: m.edge_setup_welcome_title(),
        subtitle: m.edge_setup_welcome_subtitle(),
        content: <WelcomePageContent />,
        docsLink: 'https://docs.defguard.net/edge-component/deployment',
        docsText: m.edge_setup_welcome_docs_text(),
        media: <img src={welcomeImage} alt={m.edge_setup_welcome_image_alt()} />,
        onClose: () => {
          navigate({ to: '/vpn-overview', replace: true }).then(() => {
            setTimeout(() => {
              useEdgeWizardStore.getState().reset();
            }, 100);
          });
        },
      }}
    >
      {stepsComponents[activeStep]}
    </WizardPage>
  );
};
