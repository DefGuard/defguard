import './style.scss';

import React from 'react';
import shallow from 'zustand/shallow';

import MiddleFormModal from '../../../../shared/components/layout/MiddleFormModal/MiddleFormModal';
import { useModalStore } from '../../../../shared/hooks/store/useModalStore';
import AddWebhookForm from './AddWebhookForm';

const AddWebhookModal: React.FC = () => {
  const [{ visible: isOpen }, setModalState] = useModalStore(
    (state) => [state.addWebhookModal, state.setAddWebhookModal],
    shallow
  );

  const setIsOpen = (v: boolean) => setModalState({ visible: v });

  return (
    <MiddleFormModal
      title="New webhook"
      isOpen={isOpen}
      setIsOpen={setIsOpen}
      FormComponent={AddWebhookForm}
    />
  );
};

export default AddWebhookModal;
