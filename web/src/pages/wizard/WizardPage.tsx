import './style.scss';

import { ReactNode, useEffect, useMemo } from 'react';
import { Navigate, Route, Routes, useNavigate } from 'react-router';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../i18n/i18n-react';
import { PageContainer } from '../../shared/components/layout/PageContainer/PageContainer';
import { useNavigationStore } from '../../shared/hooks/store/useNavigationStore';
import { WizardMapDevices } from './components/WizardMapDevices/WizardMapDevices';
import { WizardNav } from './components/WizardNav/WizardNav';
import { WizardNetworkImport } from './components/WizardNetworkImport/WizardNetworkImport';
import { WizardNetworkSetup } from './components/WizardNetworkSetup/WizardNetworkSetup';
import { WizardType } from './components/WizardType/WizardType';
import { WizardWelcome } from './components/WizardWelcome/WizardWelcome';
import { useWizardStore, WizardSetupType } from './hooks/useWizardStore';

export const WizardPage = () => {
  const navigate = useNavigate();
  const wizardEnabled = useNavigationStore((state) => state.enableWizard);

  useEffect(() => {
    if (!wizardEnabled) {
      navigate('/admin/overview', { replace: true });
    }
  }, [navigate, wizardEnabled]);

  return (
    <PageContainer id="wizard-page">
      <Routes>
        <Route index element={<WizardRender />} />
        <Route path="*" element={<Navigate replace to="" />} />
      </Routes>
    </PageContainer>
  );
};

type WizardStep = {
  title: string;
  element: ReactNode;
};

const WizardRender = () => {
  const { LL } = useI18nContext();
  const [setupType, currentStep] = useWizardStore(
    (state) => [state.setupType, state.currentStep],
    shallow
  );
  const getSteps = useMemo((): WizardStep[] => {
    let res: WizardStep[] = [
      {
        title: LL.wizard.navigation.titles.welcome(),
        element: <WizardWelcome key={0} />,
      },
      {
        title: LL.wizard.navigation.titles.choseNetworkSetup(),
        element: <WizardType key={1} />,
      },
    ];
    switch (setupType) {
      case WizardSetupType.IMPORT:
        res = [
          ...res,
          {
            title: LL.wizard.navigation.titles.importConfig(),
            element: <WizardNetworkImport key={2} />,
          },
          {
            title: LL.wizard.navigation.titles.mapDevices(),
            element: <WizardMapDevices key={4} />,
          },
        ];
        break;
      case WizardSetupType.MANUAL:
        res = [
          ...res,
          {
            title: LL.wizard.navigation.titles.manualConfig(),
            element: <WizardNetworkSetup key={3} />,
          },
        ];
        break;
    }
    return res;
  }, [LL.wizard.navigation.titles, setupType]);

  return (
    <div id="wizard-content">
      <WizardNav
        title={getSteps[currentStep].title}
        lastStep={currentStep === getSteps.length - 1}
      />
      {getSteps[currentStep].element || null}
    </div>
  );
};
