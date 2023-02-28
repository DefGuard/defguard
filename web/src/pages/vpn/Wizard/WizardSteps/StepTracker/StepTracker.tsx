import './style.scss';

import shallow from 'zustand/shallow';

import { useWizardStore } from '../store';

export const StepTracker = () => {
  const [totalSteps, currentStep] = useWizardStore(
    (state) => [state.stepsCount, state.currentStep],
    shallow
  );

  if (!currentStep) {
    return null;
  }

  return (
    <p className="steps-tracker">
      Step {currentStep} <span>of {totalSteps}</span>
    </p>
  );
};
