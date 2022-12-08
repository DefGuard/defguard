import './style.scss';

import { ReactNode } from 'react';

import { ModalWithTitle } from '../../../../../shared/components/layout/ModalWithTitle/ModalWithTitle';
import { useModalStore } from '../../../../../shared/hooks/store/useModalStore';
import { SetupStep } from './steps/SetupStep';

const modalSteps: ReactNode[] = [<SetupStep key={0} />];

export const UserDeviceModal = () => {
  const modalState = useModalStore((state) => state.userDeviceModal);
  const setModalState = useModalStore((state) => state.setUserDeviceModal);
  return (
    <ModalWithTitle
      title="Add device"
      isOpen={modalState.visible}
      setIsOpen={(visibility) =>
        setModalState({ visible: visibility, currentStep: 0 })
      }
      id="add-user-device-modal"
      steps={modalSteps}
      currentStep={modalState.currentStep}
      backdrop
    />
  );
};
