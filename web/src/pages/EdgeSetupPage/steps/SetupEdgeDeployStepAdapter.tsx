import { SetupEdgeDeployStep } from '../../../shared/components/wizard-steps/SetupEdgeDeployStep/SetupEdgeDeployStep';
import { EdgeSetupStep } from '../types';
import { useEdgeWizardStore } from '../useEdgeWizardStore';

export const SetupEdgeDeployStepAdapter = () => {
  const setIsOnWelcomePage = useEdgeWizardStore((s) => s.setIsOnWelcomePage);
  const setActiveStep = useEdgeWizardStore((s) => s.setActiveStep);

  return (
    <SetupEdgeDeployStep
      onBack={() => {
        setIsOnWelcomePage(true);
      }}
      onNext={() => {
        setActiveStep(EdgeSetupStep.EdgeComponent);
      }}
    />
  );
};
