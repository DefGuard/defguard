import { useShallow } from 'zustand/react/shallow';
import { SetupEdgeDeployStep } from '../../../shared/components/wizard-steps/SetupEdgeDeployStep/SetupEdgeDeployStep';
import { useMigrationWizardStore } from '../store/useMigrationWizardStore';

export const MigrationWizardEdgeDeploymentStepAdapter = () => {
  const [back, next] = useMigrationWizardStore(useShallow((s) => [s.back, s.next]));
  return (
    <SetupEdgeDeployStep
      onNext={() => {
        next();
      }}
      onBack={() => {
        back();
      }}
    />
  );
};
