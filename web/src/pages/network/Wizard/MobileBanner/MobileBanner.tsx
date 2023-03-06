import './style.scss';

import { isUndefined } from 'lodash-es';

import SvgDefguadNavLogo from '../../../../shared/components/svg/DefguadNavLogo';
import { useWizardStore } from '../WizardSteps/store';

export const MobileBanner = () => {
  const currentStep = useWizardStore((state) => state.currentStep);

  return (
    <div className="mobile-banner">
      <SvgDefguadNavLogo />
      {isUndefined(currentStep) && (
        <p className="welcome-message">Network setup</p>
      )}
    </div>
  );
};
