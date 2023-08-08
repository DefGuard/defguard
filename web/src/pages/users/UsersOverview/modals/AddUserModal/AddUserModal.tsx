import './style.scss';

import React from 'react';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { ModalWithTitle } from '../../../../../shared/defguard-ui/components/Layout/modals/ModalWithTitle/ModalWithTitle';
import { useModalStore } from '../../../../../shared/hooks/store/useModalStore';
import { AddUserForm } from './AddUserForm';

const AddUserModal: React.FC = () => {
  const [{ visible: isOpen }, setModalState] = useModalStore(
    (state) => [state.addUserModal, state.setAddUserModal],
    shallow,
  );

  const setIsOpen = (v: boolean) => setModalState({ visible: v });
  const { LL } = useI18nContext();

  return (
    <ModalWithTitle
      backdrop
      title={LL.modals.addUser.title()}
      isOpen={isOpen}
      setIsOpen={setIsOpen}
      id="add-user-modal"
    >
      <AddUserForm />
    </ModalWithTitle>
  );
};

export default AddUserModal;
