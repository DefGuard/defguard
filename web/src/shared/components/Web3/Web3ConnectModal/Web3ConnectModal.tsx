import './style.scss';

import { ReactNode } from 'react';
import { useConnect } from 'wagmi';

import { useModalStore } from '../../../hooks/store/useModalStore';
import { useToaster } from '../../../hooks/useToaster';
import { Button } from '../../layout/Button/Button';
import { ButtonSize } from '../../layout/Button/types';
import { ModalWithTitle } from '../../layout/ModalWithTitle/ModalWithTitle';
import { RowBox } from '../../layout/RowBox/RowBox';
import { MetamaskIcon, WalletconnectIcon } from '../../svg';

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
          size={ButtonSize.LARGE}
          onClick={() => setModalsState({ connectWalletModal: { visible: false } })}
        />
      </div>
    </ModalWithTitle>
  );
};

const WalletConnectorsList = () => {
  const modalState = useModalStore((state) => state.connectWalletModal);
  const setModalsStore = useModalStore((state) => state.setState);
  const { connectAsync, connectors, isLoading, pendingConnector } = useConnect();
  const toaster = useToaster();

  return (
    <div className="connectors">
      {connectors.map((connector) => (
        <RowBox
          key={connector.id}
          onClick={async () => {
            try {
              await connectAsync({ connector });
            } catch (err) {
              if (err) {
                toaster.error('Failed to connect wallet.');
                console.error(err);
                return;
              }
            }

            if (modalState.onConnect) {
              modalState.onConnect();
            }

            toaster.success('Wallet connected.');
            setModalsStore({
              connectWalletModal: { visible: false, onConnect: undefined },
            });
          }}
          disabled={
            isLoading || connector.id === pendingConnector?.id || !connector.ready
          }
        >
          {getConnectorIcon(connector.name)}
          <p>{connector.name}</p>
        </RowBox>
      ))}
    </div>
  );
};
