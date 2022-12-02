import './style.scss';

import React from 'react';
import shallow from 'zustand/shallow';

import { ModalWithTitle } from '../../../../shared/components/layout/ModalWithTitle/ModalWithTitle';
import { useModalStore } from '../../../../shared/hooks/store/useModalStore';
import AddOpenidClientForm from './AddOpenidClientForm';

const AddOpenidClientModal: React.FC = () => {
  const [{ visible: isOpen }, setModalState] = useModalStore(
    (state) => [state.addOpenidClientModal, state.setAddOpenidClientModal],
    shallow
  );

  const setIsOpen = (v: boolean) => setModalState({ visible: v });

  return (
    <ModalWithTitle
      title="New app"
      isOpen={isOpen}
      setIsOpen={setIsOpen}
      id="add-client-modal"
      backdrop
    >
      <AddOpenidClientForm />
    </ModalWithTitle>
  );
};

export default AddOpenidClientModal;
