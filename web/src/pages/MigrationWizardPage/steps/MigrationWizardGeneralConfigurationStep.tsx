import { m } from '../../../paraglide/messages';
import { Controls } from '../../../shared/components/Controls/Controls';
import { WizardCard } from '../../../shared/components/wizard/WizardCard/WizardCard';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { useMigrationWizardStore } from '../store/useMigrationWizardStore';
import { MigrationWizardStep } from '../types';

export const MigrationWizardGeneralConfigurationStep = () => {
  const setActiveStep = useMigrationWizardStore((s) => s.setActiveStep);

  return (
    <WizardCard>
      <Controls>
        <div className="right">
          <Button
            text={m.controls_continue()}
            onClick={() => setActiveStep(MigrationWizardStep.Ca)}
          />
        </div>
      </Controls>
    </WizardCard>
  );
};
