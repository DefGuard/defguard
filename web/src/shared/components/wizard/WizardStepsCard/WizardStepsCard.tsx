import { Fragment } from 'react/jsx-runtime';
import type { WizardPageStep } from '../types';
import './style.scss';
import clsx from 'clsx';
import { Icon } from '../../../defguard-ui/components/Icon';

interface Props {
  steps: WizardPageStep[];
  activeStep: number;
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
                muted: step.id > activeStep,
                active: step.id === activeStep,
                success: step.id < activeStep,
              })}
            >
              <div className={clsx('step-indicator')}>
                <div className="circle"></div>
                {step.id < activeStep && <Icon icon="check" size={12} />}
                {step.id >= activeStep && <span>{step.id + 1}</span>}
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
