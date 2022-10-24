import { chain, configureChains, createClient, WagmiConfig } from 'wagmi';
import { InjectedConnector } from 'wagmi/connectors/injected';
import { WalletConnectConnector } from 'wagmi/connectors/walletConnect';
import { infuraProvider } from 'wagmi/providers/infura';

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

interface Props {
  children?: React.ReactNode;
}

const WagmiProvider: React.FC<Props> = ({ children }) => {
  return <WagmiConfig client={client}>{children}</WagmiConfig>;
};

export default WagmiProvider;
