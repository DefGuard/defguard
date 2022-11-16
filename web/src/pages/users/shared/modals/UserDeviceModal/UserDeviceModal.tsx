import './style.scss';

import { useMemo } from 'react';

import MessageBox, {
  MessageBoxType,
} from '../../../../../shared/components/layout/MessageBox/MessageBox';
import { ModalWithTitle } from '../../../../../shared/components/layout/ModalWithTitle/ModalWithTitle';
import { useModalStore } from '../../../../../shared/hooks/store/useModalStore';
import { UserDeviceModalForm } from './UserDeviceModalForm';

export const UserDeviceModal = () => {
  const modalState = useModalStore((state) => state.userDeviceModal);
  const setModalState = useModalStore((state) => state.setUserDeviceModal);
  const editMode = useMemo(() => {
    if (modalState.device) {
      if (
        modalState.device.name &&
        modalState.device.wireguard_pubkey &&
        modalState.username
      ) {
        return true;
      }
    }
    return false;
  }, [modalState.device, modalState.username]);

  return (
    <ModalWithTitle
      title={editMode ? `Edit device ${modalState.device?.name}` : 'Add device'}
      isOpen={modalState.visible}
      setIsOpen={(visibility) => setModalState({ visible: visibility })}
      className="user-device-modal"
      backdrop
    >
      <MessageBox type={MessageBoxType.INFO}>
        <p>
          You need to configure WireguardVPN on your device, please visit{' '}
          <a href="" target="blank">
            documentation
          </a>
          if you don&apos;t know how to do it.
        </p>
      </MessageBox>
      <UserDeviceModalForm />
    </ModalWithTitle>
  );
};
