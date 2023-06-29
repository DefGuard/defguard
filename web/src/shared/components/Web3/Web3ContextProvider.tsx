import { ReactNode } from 'react';
import { configureChains, createClient, mainnet, WagmiConfig } from 'wagmi';
import { MetaMaskConnector } from 'wagmi/connectors/metaMask';
import { publicProvider } from 'wagmi/providers/public';

import { Web3ConnectModal } from './Web3ConnectModal/Web3ConnectModal';

const { chains, provider, webSocketProvider } = configureChains(
  [mainnet],
  [publicProvider()]
);

const wagmiClient = createClient({
  provider,
  webSocketProvider,
  autoConnect: true,
  connectors: [new MetaMaskConnector({ chains })],
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
