import './style.scss';
import { useNavigate } from '@tanstack/react-router';
import { type ReactNode, useMemo } from 'react';
import { m } from '../../paraglide/messages';
import { Controls } from '../../shared/components/Controls/Controls';
import type { WizardPageStep } from '../../shared/components/wizard/types';
import { WizardCoverImage } from '../../shared/components/wizard/WizardCoverImage/WizardCoverImage';
import { WizardPage } from '../../shared/components/wizard/WizardPage/WizardPage';
import { Button } from '../../shared/defguard-ui/components/Button/Button';
import { Divider } from '../../shared/defguard-ui/components/Divider/Divider';
import { ThemeSpacing } from '../../shared/defguard-ui/types';
import { SetupConfirmationStep } from './steps/SetupConfirmationStep';
import { SetupEdgeAdoptionStep } from './steps/SetupEdgeAdoptionStep';
import { SetupEdgeComponentStep } from './steps/SetupEdgeComponentStep';
import { SetupEdgeDeployStepAdapter } from './steps/SetupEdgeDeployStepAdapter';
import { EdgeSetupStep, type EdgeSetupStepValue } from './types';
import { useEdgeWizardStore } from './useEdgeWizardStore';

export const EdgeSetupPage = () => {
  const activeStep = useEdgeWizardStore((s) => s.activeStep);
  const isOnWelcomePage = useEdgeWizardStore((s) => s.isOnWelcomePage);
  const setIsOnWelcomePage = useEdgeWizardStore((s) => s.setIsOnWelcomePage);
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
      edgeDeploy: <SetupEdgeDeployStepAdapter />,
      edgeComponent: <SetupEdgeComponentStep />,
      edgeAdoption: <SetupEdgeAdoptionStep />,
      confirmation: <SetupConfirmationStep />,
    }),
    [],
  );

  const WelcomePageContent = () => (
    <>
      <Divider spacing={ThemeSpacing.Xl2} />
      <Controls>
        <Button
          text={m.edge_setup_controls_configure()}
          onClick={() => setIsOnWelcomePage(false)}
        />
      </Controls>
    </>
  );

  return (
    <WizardPage
      id="edge-setup-wizard"
      activeStep={activeStep}
      subtitle={m.edge_setup_page_subtitle()}
      title={m.edge_setup_page_title()}
      steps={stepsConfig}
      isOnWelcomePage={isOnWelcomePage}
      onClose={() => {
        navigate({ to: '/vpn-overview', replace: true }).then(() => {
          setTimeout(() => {
            useEdgeWizardStore.getState().reset();
          }, 100);
        });
      }}
      welcomePageConfig={{
        title: m.edge_setup_welcome_title(),
        subtitle: m.edge_setup_welcome_subtitle(),
        content: <WelcomePageContent />,
        docsLink: 'https://docs.defguard.net/edge-component/deployment',
        docsText: m.edge_setup_welcome_docs_text(),
        media: <WizardCoverImage variant="edge" />,
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
