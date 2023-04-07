import React, { ReactNode } from 'react';
import { Navigate } from 'react-router-dom';

import { useWizardStore } from '../store';

interface Props {
  targetStep: number;
  children?: ReactNode;
}

const StepGuard: React.FC<Props> = ({ targetStep, children }) => {
  const formStatus = useWizardStore((state) => state.formStatus);
  for (let i = 1; i < targetStep; i++) {
    if (!formStatus[i]) {
      return <Navigate replace to={`${i}`} />;
    }
  }
  return <>{children}</>;
};

export default StepGuard;
