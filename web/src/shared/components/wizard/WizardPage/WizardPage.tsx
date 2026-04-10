import {
  type HTMLProps,
  type PropsWithChildren,
  Suspense,
  useEffect,
  useMemo,
} from 'react';
import './style.scss';
import clsx from 'clsx';
import { orderBy } from 'lodash-es';
import Skeleton from 'react-loading-skeleton';
import { Badge } from '../../../defguard-ui/components/Badge/Badge';
import { SizedBox } from '../../../defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../defguard-ui/types';
import { isPresent } from '../../../defguard-ui/utils/isPresent';
import { useWizardVideoGuidePlacement } from '../../../video-tutorials/resolved';
import { LayoutGrid } from '../../LayoutGrid/LayoutGrid';
import type { WizardPageConfig } from '../types';
import { WizardStepsCard } from '../WizardStepsCard/WizardStepsCard';
import { WizardTop } from '../WizardTop/WizardTop';
import { WizardVideoGuide } from '../WizardVideoGuide/WizardVideoGuide';
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
  videoGuidePlacementKey,
  children,
  onClose,
  welcomePageConfig,
  isOnWelcomePage = false,
  ...containerProps
}: Props) => {
  const activeStep = steps[activeStepId];
  const videoGuide = useWizardVideoGuidePlacement(videoGuidePlacementKey);

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
                  {isPresent(videoGuide) && <WizardVideoGuide videoGuide={videoGuide} />}
                </div>
                <div className="main">
                  <Badge variant="success" text={`Step ${activeStepIndex + 1}`} />
                  <SizedBox height={ThemeSpacing.Md} />
                  <p className="step-title">{activeStep.label}</p>
                  {isPresent(activeStep.description) && (
                    <p className="step-description">{activeStep.description}</p>
                  )}
                  <SizedBox height={ThemeSpacing.Xl2} />
                  <Suspense fallback={<WizardStepSkeleton />}>{children}</Suspense>
                </div>
              </LayoutGrid>
            </div>
          </div>
        </>
      )}
    </div>
  );
};

const WizardStepSkeleton = () => {
  return (
    <Skeleton containerClassName="wizard-step-skeleton" width={`100%`} height={770} />
  );
};
