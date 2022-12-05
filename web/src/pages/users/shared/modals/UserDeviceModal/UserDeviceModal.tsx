import './style.scss';

import { encode } from '@stablelib/base64';
import { generateKeyPair } from '@stablelib/x25519';
import { useMemo } from 'react';

import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../shared/components/layout/Button/Button';
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

  const generateWGKeys = () => {
    const keys = generateKeyPair();
    const pub = encode(keys.publicKey);
    const priv = encode(keys.secretKey);
    console.log({ pub, priv });
  };

  return (
    <ModalWithTitle
      title={editMode ? `Edit device ${modalState.device?.name}` : 'Add device'}
      isOpen={modalState.visible}
      setIsOpen={(visibility) => setModalState({ visible: visibility })}
      className="user-device-modal"
      backdrop
    >
      <Button
        text="Generate Keys ( to console)"
        styleVariant={ButtonStyleVariant.PRIMARY}
        size={ButtonSize.BIG}
        onClick={generateWGKeys}
      />
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
