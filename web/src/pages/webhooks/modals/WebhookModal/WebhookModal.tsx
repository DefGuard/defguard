import './style.scss';

import { isUndefined } from 'lodash-es';
import { useMemo } from 'react';

import { ModalWithTitle } from '../../../../shared/components/layout/ModalWithTitle/ModalWithTitle';
import { useModalStore } from '../../../../shared/hooks/store/useModalStore';
import { WebhookForm } from './WebhookForm';
import { useI18nContext } from '../../../../i18n/i18n-react';

export const WebhookModal = () => {
  const { LL, locale } = useI18nContext();
  const modalState = useModalStore((state) => state.webhookModal);
  const getTitle = useMemo(() => {
    if (!isUndefined(modalState.webhook)) {
      return LL.modals.webhookModal.title.editWebhook();
    }
    return LL.modals.webhookModal.title.addWebhook();
  }, [modalState.webhook, locale]);
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
