import './style.scss';

import { chain, configureChains, createClient, WagmiConfig } from 'wagmi';
import { InjectedConnector } from 'wagmi/connectors/injected';
import { WalletConnectConnector } from 'wagmi/connectors/walletConnect';
import { infuraProvider } from 'wagmi/providers/infura';
import shallow from 'zustand/shallow';

import { useModalStore } from '../../../hooks/store/useModalStore';
import IconButton from '../../layout/IconButton/IconButton';
import Modal from '../../layout/Modal/Modal';
import SvgIconHamburgerClose from '../../svg/IconHamburgerClose';
import ChangeWalletForm from './ChangeWalletForm';

const { chains, provider, webSocketProvider } = configureChains(
  [chain.mainnet, chain.rinkeby],
  [infuraProvider({ apiKey: '84842078b09946638c03157f83405213' })]
);

const client = createClient({
  autoConnect: true,
  provider,
  webSocketProvider,
  connectors: [
    new InjectedConnector({ chains }),
    new WalletConnectConnector({
      chains,
      options: {
        qrcode: true,
      },
    }),
  ],
});

const ChangeWalletModal = () => {
  const [{ visible: isOpen }, setModalValues] = useModalStore(
    (state) => [state.changeWalletModal, state.setChangeWalletModal],
    shallow
  );

  return (
    <WagmiConfig client={client}>
      <Modal
        isOpen={isOpen}
        setIsOpen={() => setModalValues({ user: undefined, visible: false })}
        className="change-wallet middle"
        backdrop
        onClose={() => client.clearState()}
      >
        <header>
          <p className="title">My wallet</p>
          <IconButton
            className="blank"
            onClick={() => setModalValues({ user: undefined, visible: false })}
          >
            <SvgIconHamburgerClose />
          </IconButton>
        </header>
        <ChangeWalletForm />
      </Modal>
    </WagmiConfig>
  );
};

export default ChangeWalletModal;
