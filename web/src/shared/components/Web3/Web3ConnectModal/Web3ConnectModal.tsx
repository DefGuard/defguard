import './style.scss';

import { useEffect } from 'react';
import { useAccount, useConnect } from 'wagmi';

import { useModalStore } from '../../../hooks/store/useModalStore';
import { toaster } from '../../../utils/toaster';
import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../layout/Button/Button';
import { ModalWithTitle } from '../../layout/ModalWithTitle/ModalWithTitle';

export const Web3ConnectModal = () => {
  const modalState = useModalStore((state) => state.connectWalletModal);
  const setModalsState = useModalStore((state) => state.setState);
  return (
    <ModalWithTitle
      backdrop
      className="wallet-connect"
      title="Connect your wallet"
      isOpen={modalState.visible}
      setIsOpen={(visibility) =>
        setModalsState({ connectWalletModal: { visible: visibility } })
      }
    >
      <WalletConnectorsList />
      <div className="controls">
        <Button
          text="cancel"
          className="cancel"
          size={ButtonSize.BIG}
          onClick={() =>
            setModalsState({ connectWalletModal: { visible: false } })
          }
        />
      </div>
    </ModalWithTitle>
  );
};

const WalletConnectorsList = () => {
  const { isConnected } = useAccount();
  const setModalsStore = useModalStore((state) => state.setState);
  const { connect, connectors, error, isLoading, pendingConnector } =
    useConnect();

  useEffect(() => {
    if (error && error.message) {
      toaster.error(error.message);
      console.error(error);
    }
  }, [error]);

  useEffect(() => {
    if (isConnected) {
      setModalsStore({ connectWalletModal: { visible: false } });
      toaster.success('Wallet connected.');
    }
  }, [isConnected, setModalsStore]);

  return (
    <div className="connectors">
      {connectors.map((connector) => (
        <Button
          key={connector.id}
          text={connector.name}
          onClick={() => connect({ connector })}
          size={ButtonSize.BIG}
          styleVariant={ButtonStyleVariant.PRIMARY}
          loading={isLoading || connector.id === pendingConnector?.id}
          disabled={!connector.ready}
        />
      ))}
    </div>
  );
};
