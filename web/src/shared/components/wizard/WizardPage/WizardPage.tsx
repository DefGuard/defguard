import { type HTMLProps, type PropsWithChildren, useEffect } from 'react';
import './style.scss';
import clsx from 'clsx';
import { Badge } from '../../../defguard-ui/components/Badge/Badge';
import { SizedBox } from '../../../defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../defguard-ui/types';
import { isPresent } from '../../../defguard-ui/utils/isPresent';
import { LayoutGrid } from '../../LayoutGrid/LayoutGrid';
import type { WizardPageConfig } from '../types';
import { WizardStepsCard } from '../WizardStepsCard/WizardStepsCard';
import { WizardTop } from '../WizardTop/WizardTop';

type Props = HTMLProps<HTMLDivElement> &
  PropsWithChildren &
  WizardPageConfig & {
    onClose: () => void;
  };

export const WizardPage = ({
  className,
  activeStep: activeStepId,
  steps,
  subtitle,
  title,
  children,
  onClose,
  ...containerProps
}: Props) => {
  const activeStep = steps[activeStepId];

  // Warn user that if the tab with wizard is closed he's progress can be not recoverable
  useEffect(() => {
    if (!import.meta.env.DEV) {
      window.onbeforeunload = () => 'All unsaved changes will be lost!';

      return () => {
        window.onbeforeunload = null;
      };
    }
  }, []);

  return (
    <div className={clsx('wizard-page', className)} {...containerProps}>
      <WizardTop onClick={onClose} />
      <div className="limiter">
        <div className="content-tack">
          <LayoutGrid variant="wizard">
            <div className="side">
              <p className="title">{title}</p>
              <SizedBox height={ThemeSpacing.Md} />
              <p className="description">{subtitle}</p>
              <SizedBox height={ThemeSpacing.Xl2} />
              <WizardStepsCard steps={steps} activeStep={activeStepId} />
            </div>
            <div className="main">
              <Badge variant="success" text={`Step ${activeStepId + 1}`} />
              <SizedBox height={ThemeSpacing.Md} />
              <p className="step-title">{activeStep.label}</p>
              {isPresent(activeStep.description) && (
                <p className="step-description">{activeStep.description}</p>
              )}
              <SizedBox height={ThemeSpacing.Xl2} />
              {children}
            </div>
          </LayoutGrid>
        </div>
      </div>
    </div>
  );
};
