import './style.scss';

import { useEffect } from 'react';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../i18n/i18n-react';
import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../shared/components/layout/Button/Button';
import {
  Divider,
  DividerDirection,
} from '../../../../shared/components/layout/Divider/Divider';
import { DefguardNoIcon } from '../../../../shared/components/svg';
import SvgIconArrowGrayLeft from '../../../../shared/components/svg/IconArrowGrayLeft';
import SvgIconArrowGrayRight from '../../../../shared/components/svg/IconArrowGrayRight';
import { useNavigationStore } from '../../../../shared/hooks/store/useNavigationStore';
import { useToaster } from '../../../../shared/hooks/useToaster';
import { useWizardStore } from '../../hooks/useWizardStore';

interface Props {
  title: string;
  lastStep: boolean;
}

export const WizardNav = ({ title, lastStep }: Props) => {
  const { LL } = useI18nContext();
  const toaster = useToaster();
  const setNavigationState = useNavigationStore((state) => state.setState);
  const [backDisabled, currentStep, loading] = useWizardStore(
    (state) => [state.disableBack, state.currentStep, state.loading],
    shallow
  );
  const [back, submitSubject, next, nextSubject, resetState] = useWizardStore(
    (state) => [
      state.perviousStep,
      state.submitSubject,
      state.nextStep,
      state.nextStepSubject,
      state.resetState,
    ],
    shallow
  );

  useEffect(() => {
    const sub = nextSubject.subscribe(() => {
      if (lastStep) {
        toaster.success(LL.wizard.completed());
        setNavigationState({ enableWizard: false });
        resetState();
      } else {
        next();
      }
    });
    return () => sub?.unsubscribe();
  }, [LL.wizard, lastStep, next, nextSubject, resetState, setNavigationState, toaster]);

  if (currentStep === 0) return null;

  return (
    <div id="wizard-nav">
      <div className="top">
        <DefguardNoIcon /> <Divider direction={DividerDirection.VERTICAL} />
        <span>{LL.wizard.navigation.top()}</span>
      </div>
      <div className="bottom">
        <h1>{title}</h1>
        <div className="controls">
          <Button
            data-testid="back"
            onClick={() => back()}
            size={ButtonSize.BIG}
            text="Back"
            icon={<SvgIconArrowGrayLeft />}
            disabled={backDisabled || loading}
          />
          <Button
            data-testid="next"
            text={lastStep ? 'Finish' : 'Next'}
            size={ButtonSize.BIG}
            styleVariant={ButtonStyleVariant.PRIMARY}
            icon={!lastStep ? <SvgIconArrowGrayRight /> : null}
            loading={loading}
            onClick={() => submitSubject?.next()}
          />
        </div>
      </div>
    </div>
  );
};
