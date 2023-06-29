import './style.scss';

import { ReactNode } from 'react';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { ModalWithTitle } from '../../../../../../shared/components/layout/ModalWithTitle/ModalWithTitle';
import { useDeviceModal } from '../../hooks/useDeviceModal';
import { ConfigStep } from './steps/ConfigStep';
import { SetupStep } from './steps/SetupStep';

const modalSteps: ReactNode[] = [<SetupStep key={0} />, <ConfigStep key={1} />];

export const UserDeviceModal = () => {
  const [visible, currentStep] = useDeviceModal(
    (state) => [state.visible, state.currentStep],
    shallow
  );
  const { LL } = useI18nContext();
  const setDeviceModal = useDeviceModal((state) => state.setState);
  return (
    <ModalWithTitle
      title={LL.modals.addDevice.web.title()}
      isOpen={visible}
      setIsOpen={() => setDeviceModal({ visible: false })}
      id="add-user-device-modal"
      steps={modalSteps}
      currentStep={currentStep}
      backdrop
    />
  );
};
