import './style.scss';
import './style.scss';

import { ReactNode, useEffect } from 'react';
import { useAccount, useConnect } from 'wagmi';

import { useModalStore } from '../../../hooks/store/useModalStore';

import Button, { ButtonSize } from '../../layout/Button/Button';
import { ModalWithTitle } from '../../layout/ModalWithTitle/ModalWithTitle';
import { RowBox } from '../../layout/RowBox/RowBox';
import { MetamaskIcon, WalletconnectIcon } from '../../svg';
import { useToaster } from '../../../hooks/useToaster';

const getConnectorIcon = (name: string): ReactNode => {
  switch (name) {
    case 'WalletConnect':
      return <WalletconnectIcon />;
    case 'MetaMask':
      return <MetamaskIcon />;
    default:
      return <></>;
  }
};

export const Web3ConnectModal = () => {
  const modalState = useModalStore((state) => state.connectWalletModal);
  const setModalsState = useModalStore((state) => state.setState);
  return (
    <ModalWithTitle
      backdrop
      id="connect-wallet"
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
  const modalState = useModalStore((state) => state.connectWalletModal);
  const setModalsStore = useModalStore((state) => state.setState);
  const toaster = useToaster();
  const { connect, connectors, error, isLoading, pendingConnector } =
    useConnect();

  useEffect(() => {
    if (error && error.message) {
      toaster.error(error.message);
      console.error(error);
      setModalsStore({ connectWalletModal: { visible: false } });
    }
  }, [error, setModalsStore]);

  useEffect(() => {
    if (isConnected && modalState.visible) {
      setModalsStore({ connectWalletModal: { visible: false } });
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isConnected]);

  return (
    <div className="connectors">
      {connectors.map((connector) => (
        <RowBox
          key={connector.id}
          onClick={() => connect({ connector })}
          disabled={
            isLoading ||
            connector.id === pendingConnector?.id ||
            !connector.ready
          }
        >
          {getConnectorIcon(connector.name)}
          <p>{connector.name}</p>
        </RowBox>
      ))}
    </div>
  );
};
