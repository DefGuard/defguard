import { WalletProvider } from '../../../types';

interface Props {
  item: WalletProvider;
}

const WalletProviderListItemUnavailable: React.FC<Props> = ({ item }) => {
  return (
    <div
      key={item.title}
      className={`wallet-provider-list-item ${item.active ? '' : 'disabled'}`}
    >
      <div className="wallet-provider-list-item-icon">
        <item.Icon />
      </div>
      <div
        className={`wallet-provider-list-item-text ${
          item.active ? '' : 'disabled'
        }`}
      >
        {item.title}
      </div>
      {item.right ? (
        <div className="wallet-provider-list-item-right">{item.right}</div>
      ) : null}
    </div>
  );
};

export default WalletProviderListItemUnavailable;
