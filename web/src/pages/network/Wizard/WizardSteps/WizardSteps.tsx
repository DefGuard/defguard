import { isUndefined } from 'lodash-es';
import React, { useEffect, useMemo } from 'react';
import { useParams } from 'react-router';
import { Navigate } from 'react-router-dom';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { NetworkImport } from './NetworkImport/NetworkImport';
import { NetworkSetup } from './NetworkSetup/NetworkSetup';
import StepGuard from './StepGuard/StepGuard';
import { useWizardStore } from './store';
import { UserDevices } from './UserDevices/UserDevices';
import WizardNav from './WizardNav/WizardNav';
import { WizardType } from './WizardType/WizardType';

const stepsCount = 3;

const WizardSteps: React.FC = () => {
  const { step } = useParams();
  const { LL } = useI18nContext();

  const [setState, type] = useWizardStore((state) => {
    return [state.setState, state.type];
  });
  const formStatus = useWizardStore((state) => state.formStatus);
  const getNavTitle = useMemo(() => {
    const currentStep = Number(step);
    switch (currentStep) {
      case 1:
        return LL.wizard.navigation.titles.step1();
      case 2:
        if (!type) {
          return '';
        }
        return LL.wizard.navigation.titles.step2();
      case 3:
        if (!type) {
          return '';
        }
        return LL.wizard.navigation.titles.step3();
      default:
        return '';
    }
  }, [type, step, LL]);

  const getStepForm = useMemo(() => {
    switch (Number(step)) {
      case 1:
        return <WizardType formId={1} />;
      case 2:
        return (
          <StepGuard targetStep={2}>
            {type === 'manual' ? (
              <NetworkSetup formId={2} />
            ) : (
              <NetworkImport formId={2} />
            )}
          </StepGuard>
        );
      case 3:
        return (
          <StepGuard targetStep={3}>
            <UserDevices formId={3} />
          </StepGuard>
        );
      default:
        for (let i = 1; i <= stepsCount; i++) {
          if (!formStatus[i]) {
            return <Navigate to={`${i}`} />;
          }
        }
        return <Navigate to={String(stepsCount)} />;
    }
  }, [formStatus, step, type]);

  useEffect(() => {
    if (isUndefined(step)) {
      setState({ currentStep: undefined });
    } else {
      setState({ currentStep: Number(step) });
    }
  }, [setState, step]);

  return (
    <>
      <WizardNav
        currentStep={Number(step)}
        steps={stepsCount}
        title={getNavTitle}
      />
      {getStepForm}
    </>
  );
};

export default WizardSteps;
