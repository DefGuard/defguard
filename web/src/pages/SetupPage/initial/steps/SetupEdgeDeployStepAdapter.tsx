import { SetupEdgeDeployStep } from '../../../../shared/components/wizard-steps/SetupEdgeDeployStep/SetupEdgeDeployStep';
import { SetupPageStep } from '../types';
import { useSetupWizardStore } from '../useSetupWizardStore';

export const SetupEdgeDeployStepAdapter = () => {
  const setActiveStep = useSetupWizardStore((s) => s.setActiveStep);
  return (
    <SetupEdgeDeployStep
      onBack={() => {
        setActiveStep(SetupPageStep.CASummary);
      }}
      onNext={() => {
        setActiveStep(SetupPageStep.EdgeComponent);
      }}
    />
  );
};
