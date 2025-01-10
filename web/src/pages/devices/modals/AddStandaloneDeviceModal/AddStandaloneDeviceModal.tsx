import './style.scss';

import { useEffect, useMemo } from 'react';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { ModalWithTitle } from '../../../../shared/defguard-ui/components/Layout/modals/ModalWithTitle/ModalWithTitle';
import { FinishCliStep } from './steps/FinishCliStep/FinishCliStep';
import { FinishManualStep } from './steps/FinishManualStep/FinishManualStep';
import { MethodStep } from './steps/MethodStep/MethodStep';
import { SetupCliStep } from './steps/SetupCliStep/SetupCliStep';
import { SetupManualStep } from './steps/SetupManualStep/SetupManualStep';
import { useAddStandaloneDeviceModal } from './store';

const steps = [
  <MethodStep key={0} />,
  <SetupCliStep key={1} />,
  <FinishCliStep key={2} />,
  <SetupManualStep key={3} />,
  <FinishManualStep key={4} />,
];

export const AddStandaloneDeviceModal = () => {
  const { LL } = useI18nContext();
  const localLL = LL.modals.addStandaloneDevice;
  const [currentStep, visible] = useAddStandaloneDeviceModal(
    (s) => [s.currentStep, s.visible],
    shallow,
  );
  const [close, reset] = useAddStandaloneDeviceModal((s) => [s.close, s.reset], shallow);

  const getTitle = useMemo(() => {
    switch (currentStep.valueOf()) {
      case 0:
        return localLL.steps.method.title();
      case 1:
        return localLL.steps.cli.title();
      case 2:
        return localLL.steps.cli.title();
      case 3:
        return localLL.steps.manual.title();
      case 4:
        return localLL.steps.manual.title();
    }
  }, [currentStep, localLL.steps.cli, localLL.steps.manual, localLL.steps.method]);

  useEffect(() => {
    return () => {
      reset();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <ModalWithTitle
      id="add-standalone-device-modal"
      isOpen={visible}
      steps={steps}
      currentStep={currentStep}
      title={getTitle}
      onClose={() => close()}
      afterClose={() => reset()}
    />
  );
};
