import './style.scss';

import { isUndefined } from 'lodash-es';
import { useMemo } from 'react';

import { ModalWithTitle } from '../../../../shared/components/layout/ModalWithTitle/ModalWithTitle';
import { useModalStore } from '../../../../shared/hooks/store/useModalStore';
import { WebhookForm } from './WebhookForm';

export const WebhookModal = () => {
  const modalState = useModalStore((state) => state.webhookModal);
  const getTitle = useMemo(() => {
    if (!isUndefined(modalState.webhook)) {
      return 'Edit webhook';
    }
    return 'Add webhook';
  }, [modalState.webhook]);
  const setModalState = useModalStore((state) => state.setWebhookModal);
  return (
    <ModalWithTitle
      title={getTitle}
      isOpen={modalState.visible}
      setIsOpen={(v) => setModalState({ visible: v })}
      id="webhook-modal"
      backdrop
    >
      <WebhookForm />
    </ModalWithTitle>
  );
};
