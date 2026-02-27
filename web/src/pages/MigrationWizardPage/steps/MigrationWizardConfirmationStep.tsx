import { m } from '../../../paraglide/messages';
import { Controls } from '../../../shared/components/Controls/Controls';
import { WizardCard } from '../../../shared/components/wizard/WizardCard/WizardCard';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { useMigrationWizardStore } from '../store/useMigrationWizardStore';
import { MigrationWizardStep } from '../types';

export const MigrationWizardConfirmationStep = () => {
  const setActiveStep = useMigrationWizardStore((s) => s.setActiveStep);

  return (
    <WizardCard>
      <Controls>
        <Button
          variant="outlined"
          text={m.controls_back()}
          onClick={() => setActiveStep(MigrationWizardStep.EdgeAdoption)}
        />
      </Controls>
    </WizardCard>
  );
};
