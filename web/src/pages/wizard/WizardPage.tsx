import './style.scss';

import { ReactNode, useEffect, useMemo } from 'react';
import { Navigate, Route, Routes } from 'react-router';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../i18n/i18n-react';
import { PageContainer } from '../../shared/components/layout/PageContainer/PageContainer';
import { useAppStore } from '../../shared/hooks/store/useAppStore';
import { WizardMapDevices } from './components/WizardMapDevices/WizardMapDevices';
import { WizardNav } from './components/WizardNav/WizardNav';
import { WizardNetworkConfiguration } from './components/WizardNetworkConfiguration/WizardNetworkConfiguration';
import { WizardNetworkImport } from './components/WizardNetworkImport/WizardNetworkImport';
import { WizardType } from './components/WizardType/WizardType';
import { WizardWelcome } from './components/WizardWelcome/WizardWelcome';
import { useWizardStore, WizardSetupType } from './hooks/useWizardStore';

export const WizardPage = () => {
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
  className: string;
  backDisabled?: boolean;
};

const WizardRender = () => {
  const { LL } = useI18nContext();
  const networkPresent = useAppStore((state) => state.appInfo?.network_present);
  const setWizardState = useWizardStore((state) => state.setState);
  const [setupType, currentStep] = useWizardStore(
    (state) => [state.setupType, state.currentStep],
    shallow
  );
  const getSteps = useMemo((): WizardStep[] => {
    let res: WizardStep[] = [
      {
        title: LL.wizard.navigation.titles.welcome(),
        element: <WizardWelcome key={0} />,
        className: 'welcome',
      },
      {
        title: LL.wizard.navigation.titles.choseNetworkSetup(),
        element: <WizardType key={1} />,
        backDisabled: networkPresent,
        className: 'setup-selection',
      },
    ];
    switch (setupType) {
      case WizardSetupType.IMPORT:
        res = [
          ...res,
          {
            title: LL.wizard.navigation.titles.importConfig(),
            element: <WizardNetworkImport key={2} />,
            className: 'import-config',
          },
          {
            title: LL.wizard.navigation.titles.mapDevices(),
            element: <WizardMapDevices key={4} />,
            className: 'map-devices',
            backDisabled: true,
          },
        ];
        break;
      case WizardSetupType.MANUAL:
        res = [
          ...res,
          {
            title: LL.wizard.navigation.titles.manualConfig(),
            element: <WizardNetworkConfiguration key={3} />,
            className: 'network-config',
          },
        ];
        break;
    }
    return res;
  }, [LL.wizard.navigation.titles, networkPresent, setupType]);

  // skip welcome step when at least one network is already present
  useEffect(() => {
    if (networkPresent && currentStep === 0) {
      setWizardState({ currentStep: 1 });
    }
  }, [currentStep, networkPresent, setWizardState]);

  return (
    <div id="wizard-content" className={getSteps[currentStep]?.className}>
      <WizardNav
        title={getSteps[currentStep]?.title}
        lastStep={currentStep === getSteps.length - 1}
        backDisabled={getSteps[currentStep].backDisabled ?? false}
      />
      {getSteps[currentStep].element || null}
    </div>
  );
};
