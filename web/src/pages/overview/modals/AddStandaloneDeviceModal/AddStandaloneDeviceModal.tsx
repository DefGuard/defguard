import './style.scss';

import { ReactNode, useEffect, useMemo } from 'react';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { ModalWithTitle } from '../../../../shared/defguard-ui/components/Layout/modals/ModalWithTitle/ModalWithTitle';
import { FinishCliStep } from './steps/FinishCliStep/FinishCliStep';
import { MethodStep } from './steps/MethodStep/MethodStep';
import { SetupCliStep } from './steps/SetupCliStep/SetupCliStep';
import { useAddStandaloneDeviceModal } from './store';

export const AddStandaloneDeviceModal = () => {
  const { LL } = useI18nContext();
  const localLL = LL.modals.addStandaloneDevice;
  const [currentStep] = useAddStandaloneDeviceModal((s) => [s.currentStep], shallow);
  const [close, reset] = useAddStandaloneDeviceModal((s) => [s.close, s.reset], shallow);
  const steps = useMemo(
    (): ReactNode[] => [
      <MethodStep key={0} />,
      <SetupCliStep key={1} />,
      <FinishCliStep key={2} />,
    ],
    [],
  );

  const getTitle = useMemo(() => {
    switch (currentStep) {
      case 0:
        return localLL.steps.method.title();
      case 1:
        return localLL.steps.cli.title();
      case 2:
        return localLL.steps.cli.title();
    }
  }, [currentStep, localLL.steps.cli, localLL.steps.method]);

  useEffect(() => {
    return () => {
      reset();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <ModalWithTitle
      id="add-standalone-device-modal"
      isOpen={true}
      steps={steps}
      currentStep={currentStep}
      title={getTitle}
      onClose={() => close()}
      afterClose={() => reset()}
    />
  );
};
