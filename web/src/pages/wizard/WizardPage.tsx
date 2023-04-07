import './style.scss';

import { Fragment } from 'react';
import { Navigate, Route, Routes } from 'react-router';

import PageContainer from '../../shared/components/layout/PageContainer/PageContainer';
import { WizardWelcome } from './components/WizardWelcome/WizardWelcome';
import { useWizardStore } from './WizardSteps/store';

export const WizardPage = () => {
  return (
    <PageContainer id="wizard-page">
      <Routes>
        <Route index element={<WizardContent />} />
        <Route path="*" element={<Navigate replace to="" />} />
      </Routes>
    </PageContainer>
  );
};

const WizardContent = () => {
  const currentStep = useWizardStore((state) => state.currentStep);
  return <Fragment>{currentStep === 0 && <WizardWelcome />}</Fragment>;
};
