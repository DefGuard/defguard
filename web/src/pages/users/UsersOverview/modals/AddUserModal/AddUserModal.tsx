import './style.scss';

import React from 'react';
import shallow from 'zustand/shallow';

import { ModalWithTitle } from '../../../../../shared/components/layout/ModalWithTitle/ModalWithTitle';
import { useModalStore } from '../../../../../shared/hooks/store/useModalStore';
import AddUserForm from './AddUserForm';

const AddUserModal: React.FC = () => {
  const [{ visible: isOpen }, setModalState] = useModalStore(
    (state) => [state.addUserModal, state.setAddUserModal],
    shallow
  );

  const setIsOpen = (v: boolean) => setModalState({ visible: v });

  return (
    <ModalWithTitle
      backdrop
      title="New user"
      isOpen={isOpen}
      setIsOpen={setIsOpen}
      id="add-user-modal"
    >
      <AddUserForm />
    </ModalWithTitle>
  );
};

export default AddUserModal;
