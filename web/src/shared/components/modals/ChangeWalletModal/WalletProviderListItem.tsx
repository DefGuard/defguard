import classNames from 'classnames';
import { ReactNode, useMemo } from 'react';
import { Connector, useConnect } from 'wagmi';

import { MetamaskIcon, WalletconnectIcon } from '../../svg';

interface Props {
  connector: Connector;
  children?: ReactNode;
}

const WalletProviderListItem = ({ connector, children }: Props) => {
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

  const getListItemCN = useMemo(
    () =>
      classNames('wallet-provider-list-item', {
        disabled: !connector.ready,
      }),
    [connector.ready]
  );

  const getListItemTextCN = useMemo(
    () =>
      classNames('wallet-provider-list-text', {
        disabled: !connector.ready,
      }),
    [connector.ready]
  );

  return (
    <div key={connector.name} className={getListItemCN} onClick={handleClick}>
      <div className="wallet-provider-list-item-icon">{renderIcon()}</div>
      <div className={getListItemTextCN}>{connector.name}</div>
      {children ? (
        <div className="wallet-provider-list-item-right">{children}</div>
      ) : null}
    </div>
  );
};

export default WalletProviderListItem;
