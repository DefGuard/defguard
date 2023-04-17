import './style.scss';

import { useEffect } from 'react';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { Card } from '../../../../shared/components/layout/Card/Card';
import SvgImportConfig from '../../../../shared/components/svg/ImportConfig';
import SvgManualConfig from '../../../../shared/components/svg/ManualConfig';
import { useWizardStore, WizardSetupType } from '../../hooks/useWizardStore';
import { WizardTypeOptionCard } from './components/WizardTypeOptionCard/WizardTypeOptionCard';

export const WizardType = () => {
  const { LL } = useI18nContext();
  const setupType = useWizardStore((state) => state.setupType);
  const disableNext = useWizardStore((state) => state.disableNext);
  const [setWizardState, nextStepSubject] = useWizardStore(
    (state) => [state.setState, state.nextStepSubject],
    shallow
  );
  const submitSubject = useWizardStore((state) => state.submitSubject);

  useEffect(() => {
    if (!setupType && !disableNext) {
      setWizardState({ disableNext: true });
    }
    if (setupType && disableNext) {
      setWizardState({ disableNext: false });
    }
  }, [disableNext, setWizardState, setupType]);

  useEffect(() => {
    if (submitSubject && submitSubject.subscribe) {
      const sub = submitSubject.subscribe(() => {
        nextStepSubject.next();
      });
      return () => sub?.unsubscribe();
    }
  }, [submitSubject, nextStepSubject]);

  return (
    <Card id="wizard-setup-choice" shaded>
      <WizardTypeOptionCard
        title={LL.wizard.wizardType.import.title()}
        subtitle={LL.wizard.wizardType.import.description()}
        icon={<SvgImportConfig />}
        selected={setupType === WizardSetupType.IMPORT}
        onClick={() => setWizardState({ setupType: WizardSetupType.IMPORT })}
      />
      <WizardTypeOptionCard
        title={LL.wizard.wizardType.manual.title()}
        subtitle={LL.wizard.wizardType.manual.description()}
        icon={<SvgManualConfig />}
        selected={setupType === WizardSetupType.MANUAL}
        onClick={() => setWizardState({ setupType: WizardSetupType.MANUAL })}
      />
    </Card>
  );
};
