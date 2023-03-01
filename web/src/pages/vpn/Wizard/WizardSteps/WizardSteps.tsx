import { isUndefined } from 'lodash-es';
import React, { useEffect, useMemo, useState } from 'react';
import { useI18nContext } from '../../../../i18n/i18n-react';
import { useParams } from 'react-router';
import { Navigate } from 'react-router-dom';

import { WizardNetwork } from '../types/types';
import { NetworkConfiguration } from '../../../network/NetworkConfiguration/NetworkConfiguration';
import { NetworkType } from './NetworkType/NetworkType';
import StepGuard from './StepGuard/StepGuard';
import { useWizardStore } from './store';
import WizardNav from './WizardNav/WizardNav';

const stepsCount = 2;

const WizardSteps: React.FC = () => {
  const { step } = useParams();
  const { LL } = useI18nContext();
  const networkObserver = useWizardStore((state) => state.network);
  const setWizardState = useWizardStore((state) => state.setState);
  const [network, setNetwork] = useState<WizardNetwork | undefined>();
  const formStatus = useWizardStore((state) => state.formStatus);
  const getNavTitle = useMemo(() => {
    const networkType = network?.type;
    const currentStep = Number(step);
    switch (currentStep) {
      case 1:
        return LL.wizard.navigation.titles.step1();
      case 2:
        if (!networkType) {
          return '';
        }
        return LL.wizard.navigation.titles.step2();
      case 3:
        if (!networkType) {
          return '';
        }
        return LL.wizard.navigation.titles.step3();
      default:
        return '';
    }
  }, [network, step, LL]);

  const getStepForm = useMemo(() => {
    switch (Number(step)) {
      case 1:
        return <NetworkType formId={1} />;
      case 2:
        return (
          <StepGuard targetStep={2}>
            {network?.type === 'regular' ? (
              <NetworkConfiguration />
            ) : (
              <NetworkConfiguration />
            )}
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
  }, [formStatus, step, network?.type]);

  useEffect(() => {
    if (isUndefined(step)) {
      setWizardState({ currentStep: undefined });
    } else {
      setWizardState({ currentStep: Number(step) });
    }
  }, [setWizardState, step]);

  useEffect(() => {
    setNetwork(networkObserver?.getValue());
    const sub = networkObserver?.subscribe((data) => setNetwork(data));
    return () => sub?.unsubscribe();
  }, [networkObserver]);

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
