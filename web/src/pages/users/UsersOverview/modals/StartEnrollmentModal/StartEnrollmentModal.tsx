import React from 'react';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { ModalWithTitle } from '../../../../../shared/components/layout/ModalWithTitle/ModalWithTitle';
import { useModalStore } from '../../../../../shared/hooks/store/useModalStore';
import { StartEnrollmentForm } from './StartEnrollmentForm';

const StartEnrollmentModal: React.FC = () => {
  const [{ visible: isOpen }, setModalState] = useModalStore(
    (state) => [state.startEnrollmentModal, state.setStartEnrollmentModal],
    shallow
  );

  const setIsOpen = (v: boolean) => setModalState({ visible: v });
  const { LL } = useI18nContext();

  return (
    <ModalWithTitle
      backdrop
      title={LL.modals.startEnrollment.title()}
      isOpen={isOpen}
      setIsOpen={setIsOpen}
      id="start-enrollment-modal"
    >
      <StartEnrollmentForm />
    </ModalWithTitle>
  );
};

export default StartEnrollmentModal;
