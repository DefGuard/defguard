import { Connector, useConnect } from 'wagmi';

import { MetamaskIcon, WalletconnectIcon } from '../../svg';

interface Props {
  connector: Connector;
  right?: React.ReactNode;
}

const WalletProviderListItem: React.FC<Props> = ({ connector, right }) => {
  const { connect } = useConnect();

  const handleClick = () => {
    if (connector.ready) {
      connect({ connector });
    }
  };

  const renderIcon = () => {
    if (connector.name === 'MetaMask') {
      return <MetamaskIcon />;
    }

    if (connector.name === 'WalletConnect') {
      return <WalletconnectIcon />;
    }

    return null;
  };

  return (
    <div
      key={connector.name}
      className={`wallet-provider-list-item ${
        connector.ready ? '' : 'disabled'
      }`}
      onClick={handleClick}
    >
      <div className="wallet-provider-list-item-icon">{renderIcon()}</div>
      <div
        className={`wallet-provider-list-item-text ${
          connector.ready ? '' : 'disabled'
        }`}
      >
        {connector.name}
      </div>
      {right ? (
        <div className="wallet-provider-list-item-right">{right}</div>
      ) : null}
    </div>
  );
};

export default WalletProviderListItem;
