import { ReactNode } from 'react';
import {
  configureChains,
  createClient,
  defaultChains,
  WagmiConfig,
} from 'wagmi';
import { InjectedConnector } from 'wagmi/connectors/injected';
import { WalletConnectConnector } from 'wagmi/connectors/walletConnect';
import { publicProvider } from 'wagmi/providers/public';

import { Web3ConnectModal } from './Web3ConnectModal/Web3ConnectModal';

const { chains, provider, webSocketProvider } = configureChains(defaultChains, [
  publicProvider(),
]);

const wagmiClient = createClient({
  provider,
  webSocketProvider,
  autoConnect: true,
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

interface Props {
  children?: ReactNode;
}

export const Web3ContextProvider = ({ children }: Props) => {
  return (
    <WagmiConfig client={wagmiClient}>
      {children}
      <Web3ConnectModal />
    </WagmiConfig>
  );
};
