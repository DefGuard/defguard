import './style.scss';

import React from 'react';
import shallow from 'zustand/shallow';

import MiddleFormModal from '../../../../shared/components/layout/MiddleFormModal/MiddleFormModal';
import { useModalStore } from '../../../../shared/hooks/store/useModalStore';
import { Webhook } from '../../../../shared/types';
import EditWebhookForm from './EditWebhookForm';

interface Props {
  webhook: Webhook;
}

const EditWebhookModal: React.FC<Props> = ({ webhook }) => {
  const [{ visible: isOpen }, setModalState] = useModalStore(
    (state) => [state.editWebhookModal, state.setEditWebhookModal],
    shallow
  );

  const setIsOpen = (v: boolean) => setModalState({ visible: v });

  return (
    <MiddleFormModal
      title="Edit webhook"
      isOpen={isOpen}
      setIsOpen={setIsOpen}
      FormComponent={EditWebhookForm}
      formComponentProps={{
        webhook,
      }}
    />
  );
};

export default EditWebhookModal;
