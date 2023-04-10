import './style.scss';

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
import { useWizardStore } from '../../hooks/useWizardStore';

interface Props {
  title: string;
  lastStep: boolean;
}

export const WizardNav = ({ title, lastStep }: Props) => {
  const { LL } = useI18nContext();
  const backDisabled = useWizardStore((state) => state.disableBack);
  const nextDisabled = useWizardStore((state) => state.disableNext);
  const currentStep = useWizardStore((state) => state.currentStep);
  const [back, submitSubject] = useWizardStore(
    (state) => [state.perviousStep, state.submitSubject],
    shallow
  );

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
            disabled={backDisabled}
          />
          <Button
            data-testid="next"
            text={lastStep ? 'Finish' : 'Next'}
            size={ButtonSize.BIG}
            styleVariant={ButtonStyleVariant.PRIMARY}
            icon={!lastStep ? <SvgIconArrowGrayRight /> : null}
            disabled={nextDisabled}
            onClick={() => submitSubject?.next()}
          />
        </div>
      </div>
    </div>
  );
};
