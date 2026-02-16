import { type HTMLProps, type PropsWithChildren, useEffect, useMemo } from 'react';
import './style.scss';
import clsx from 'clsx';
import { orderBy } from 'lodash-es';
import { Badge } from '../../../defguard-ui/components/Badge/Badge';
import { SizedBox } from '../../../defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../defguard-ui/types';
import { isPresent } from '../../../defguard-ui/utils/isPresent';
import { LayoutGrid } from '../../LayoutGrid/LayoutGrid';
import type { WizardPageConfig } from '../types';
import { WizardStepsCard } from '../WizardStepsCard/WizardStepsCard';
import { WizardTop } from '../WizardTop/WizardTop';
import { WizardWelcomePage } from '../WizardWelcomePage/WizardWelcomePage';

type Props = HTMLProps<HTMLDivElement> &
  PropsWithChildren &
  WizardPageConfig & {
    onClose?: () => void;
  };

export const WizardPage = ({
  className,
  activeStep: activeStepId,
  steps,
  subtitle,
  title,
  children,
  onClose,
  welcomePageConfig,
  isOnWelcomePage = false,
  ...containerProps
}: Props) => {
  const activeStep = steps[activeStepId];

  const visibleSteps = useMemo(
    () =>
      orderBy(
        Object.values(steps).filter((step) => !step.hidden),
        (s) => s.order,
        ['asc'],
      ),
    [steps],
  );

  const activeStepIndex = useMemo(
    () => visibleSteps.findIndex((s) => s.id === activeStepId),
    [visibleSteps, activeStepId],
  );

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
      {isPresent(welcomePageConfig) && isOnWelcomePage ? (
        <WizardWelcomePage {...welcomePageConfig} />
      ) : (
        <>
          <WizardTop onClick={onClose} />
          <div className="limiter">
            <div className="content-tack">
              <LayoutGrid variant="wizard">
                <div className="side">
                  <p className="title">{title}</p>
                  <SizedBox height={ThemeSpacing.Md} />
                  <p className="description">{subtitle}</p>
                  <SizedBox height={ThemeSpacing.Xl2} />
                  <WizardStepsCard steps={visibleSteps} activeStep={activeStep} />
                </div>
                <div className="main">
                  <Badge variant="success" text={`Step ${activeStepIndex + 1}`} />
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
        </>
      )}
    </div>
  );
};
