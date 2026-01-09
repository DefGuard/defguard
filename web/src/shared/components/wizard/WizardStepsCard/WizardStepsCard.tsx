import { Fragment } from 'react/jsx-runtime';
import type { WizardPageStep } from '../types';
import './style.scss';
import clsx from 'clsx';
import { Icon } from '../../../defguard-ui/components/Icon';

interface Props {
  activeStep: WizardPageStep;
  steps: WizardPageStep[];
}

export const WizardStepsCard = ({ steps, activeStep }: Props) => {
  return (
    <div className="wizard-steps-card">
      <ul>
        {steps.map((step, index) => (
          <Fragment key={step.id}>
            <li
              key={step.id}
              className={clsx({
                muted: step.order > activeStep.order,
                active: step.id === activeStep.id,
                success: step.order < activeStep.order,
              })}
            >
              <div className={clsx('step-indicator')}>
                <div className="circle"></div>
                {step.order < activeStep.order && <Icon icon="check" size={12} />}
                {step.order >= activeStep.order && <span>{index + 1}</span>}
              </div>
              <span>{step.label}</span>
            </li>
            {index !== steps.length - 1 && (
              <li className="spacer" aria-hidden>
                <div className="line"></div>
              </li>
            )}
          </Fragment>
        ))}
      </ul>
    </div>
  );
};
