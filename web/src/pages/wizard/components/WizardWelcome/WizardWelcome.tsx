import './style.scss';

import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../shared/components/layout/Button/Button';
import { Card } from '../../../../shared/components/layout/Card/Card';
import { useWizardStore } from '../../hooks/useWizardStore';

export const WizardWelcome = () => {
  const nextStep = useWizardStore((state) => state.nextStep);
  return (
    <Card id="wizard-welcome">
      <h1>Welcome to defguard!</h1>
      <p>Before you start, you need to setup your network environment first.</p>
      <Button
        onClick={() => nextStep()}
        size={ButtonSize.BIG}
        styleVariant={ButtonStyleVariant.PRIMARY}
        text="Setup my network"
      />
    </Card>
  );
};
