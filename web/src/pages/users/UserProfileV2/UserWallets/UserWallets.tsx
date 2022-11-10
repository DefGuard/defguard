import './style.scss';

import { isUndefined } from 'lodash-es';
import { alphabetical } from 'radash';
import { useEffect, useMemo } from 'react';
import { useAccount, useDisconnect } from 'wagmi';

import { useModalStore } from '../../../../shared/hooks/store/useModalStore';
import { useUserProfileV2Store } from '../../../../shared/hooks/store/useUserProfileV2Store';
import { toaster } from '../../../../shared/utils/toaster';
import AddComponentBox from '../../shared/components/AddComponentBox/AddComponentBox';
import { AddWalletModal } from './AddWalletModal/AddWalletModal';
import { WalletCard } from './WalletCard/WalletCard';

export const UserWallets = () => {
  const { isConnected, address } = useAccount();
  const { disconnect, disconnectAsync } = useDisconnect();
  const user = useUserProfileV2Store((state) => state.user);
  const isMe = useUserProfileV2Store((state) => state.isMe);
  const setModalsState = useModalStore((state) => state.setState);

  const sortedWallet = useMemo(() => {
    if (user && user.wallets) {
      return alphabetical(user.wallets, (w) => w.name);
    }
    return [];
  }, [user]);

  useEffect(() => {
    if (isConnected) {
      if (address && user) {
        const alreadyAdded = !isUndefined(
          user.wallets.find((w) => w.address === address)
        );
        if (alreadyAdded) {
          disconnect();
          toaster.error('This wallet is already registered. Disconnected.');
        } else {
          setModalsState({ addWalletModal: { visible: true } });
        }
      }
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isConnected]);

  return (
    <section id="user-wallets">
      <header>
        <h2>User wallets</h2>
      </header>
      {sortedWallet && sortedWallet.length > 0 && (
        <div className="wallets">
          {sortedWallet.map((wallet) => (
            <WalletCard
              key={wallet.address}
              wallet={wallet}
              connected={address ? wallet.address === address : false}
              showMFA={isMe}
            />
          ))}
        </div>
      )}
      <AddComponentBox
        callback={async () => {
          if (!isConnected) {
            setModalsState({
              connectWalletModal: { visible: true },
            });
            return;
          }
          if (
            isConnected &&
            user &&
            !isUndefined(user.wallets.find((w) => w.address === address))
          ) {
            await disconnectAsync();
            toaster.warning(
              'Connected wallet was already added. Wallet was disconnected !'
            );
          } else {
            setModalsState({ addWalletModal: { visible: true } });
          }
        }}
        text="Add new wallet"
      />
      <AddWalletModal />
    </section>
  );
};
