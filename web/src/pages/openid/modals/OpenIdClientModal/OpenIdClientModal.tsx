import './style.scss';

import { useMemo } from 'react';

import { ModalWithTitle } from '../../../../shared/components/layout/ModalWithTitle/ModalWithTitle';
import { useModalStore } from '../../../../shared/hooks/store/useModalStore';
import { OpenIdClientModalForm } from './OpenIdClientModalForm';

export const OpenIdClientModal = () => {
  const modalState = useModalStore((state) => state.openIdClientModal);
  const setModalState = useModalStore((state) => state.setOpenIdClientModal);

  const getTitle = useMemo(() => {
    if (modalState.viewMode && modalState.client) {
      return modalState.client.name;
    }
    if (modalState.client) {
      return `Edit ${modalState.client.name} client`;
    }
    return 'Add client';
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
