import { useConnect } from 'wagmi';

import { WalletProvider } from '../../../types';
import { GlowIcon, PhantomIcon } from '../../svg';
import WalletProviderListItem from './WalletProviderListItem';
import WalletProviderListItemUnavailable from './WalletProviderListItemUnavailable';

const items = [
  {
    title: 'Phantom',
    Icon: PhantomIcon,
    right: 'Solana',
    active: false,
  },
  {
    title: 'Glow',
    Icon: GlowIcon,
    right: 'Solana',
    active: false,
  },
  {
    title: 'Coinbase Wallet',
    Icon: GlowIcon,
    active: false,
  },
] as WalletProvider[];

const WalletProviderList: React.FC = () => {
  const { connectors } = useConnect();

  return (
    <div className="wallet-provider-list">
      {connectors.map((connector) => (
        <WalletProviderListItem key={connector.id} connector={connector} />
      ))}
      {items.map((item) => (
        <WalletProviderListItemUnavailable key={item.title} item={item} />
      ))}
    </div>
  );
};

export default WalletProviderList;
