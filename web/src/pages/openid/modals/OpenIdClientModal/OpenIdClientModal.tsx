import './style.scss';

import { useMemo } from 'react';

import { ModalWithTitle } from '../../../../shared/components/layout/ModalWithTitle/ModalWithTitle';
import { useModalStore } from '../../../../shared/hooks/store/useModalStore';
import { OpenIdClientModalForm } from './OpenIdClientModalForm';
import { useI18nContext } from '../../../../i18n/i18n-react';

export const OpenIdClientModal = () => {
  const { LL } = useI18nContext();
  const modalState = useModalStore((state) => state.openIdClientModal);
  const setModalState = useModalStore((state) => state.setOpenIdClientModal);

  const getTitle = useMemo(() => {
    if (modalState.viewMode && modalState.client) {
      return modalState.client.name;
    }
    if (modalState.client) {
      return LL.openidOverview.modals.openidClientModal.title.editApp({
        appName: modalState.client.name,
      });
    }
    return LL.openidOverview.modals.openidClientModal.title.addApp();
  }, [modalState.client, modalState.viewMode]);

  return (
    <ModalWithTitle
      title={getTitle}
      backdrop
      isOpen={modalState.visible}
      setIsOpen={(v) => setModalState({ visible: v })}
      id="openid-client-modal"
    >
      <OpenIdClientModalForm />
    </ModalWithTitle>
  );
};
