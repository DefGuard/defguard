import './style.scss';

import { ReactNode } from 'react';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { ModalWithTitle } from '../../../../../../shared/components/layout/ModalWithTitle/ModalWithTitle';
import { useModalStore } from '../../../../../../shared/hooks/store/useModalStore';
import { ConfigStep } from './steps/ConfigStep';
import { SetupStep } from './steps/SetupStep';

const modalSteps: ReactNode[] = [<SetupStep key={0} />, <ConfigStep key={1} />];

export const UserDeviceModal = () => {
  const modalState = useModalStore((state) => state.userDeviceModal);
  const setModalState = useModalStore((state) => state.setUserDeviceModal);
  const { LL } = useI18nContext();
  return (
    <ModalWithTitle
      title={LL.modals.addDevice.web.title()}
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
