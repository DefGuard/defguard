import './style.scss';

import React from 'react';
import shallow from 'zustand/shallow';

import MiddleFormModal from '../../../../shared/components/layout/MiddleFormModal/MiddleFormModal';
import { useModalStore } from '../../../../shared/hooks/store/useModalStore';
import AddOpenidClientForm from './AddOpenidClientForm';

const AddOpenidClientModal: React.FC = () => {
  const [{ visible: isOpen }, setModalState] = useModalStore(
    (state) => [state.addOpenidClientModal, state.setAddOpenidClientModal],
    shallow
  );

  const setIsOpen = (v: boolean) => setModalState({ visible: v });

  return (
    <MiddleFormModal
      title="New app"
      isOpen={isOpen}
      setIsOpen={setIsOpen}
      FormComponent={AddOpenidClientForm}
    />
  );
};

export default AddOpenidClientModal;
