import './style.scss';

import { isUndefined } from 'lodash-es';
import { alphabetical } from 'radash';
import { useMemo } from 'react';
import { useAccount } from 'wagmi';

import { useModalStore } from '../../../../shared/hooks/store/useModalStore';
import { useUserProfileV2Store } from '../../../../shared/hooks/store/useUserProfileV2Store';
import { useToaster } from '../../../../shared/hooks/useToaster';
import { AddComponentBox } from '../../shared/components/AddComponentBox/AddComponentBox';
import { AddWalletModal } from './AddWalletModal/AddWalletModal';
import { WalletCard } from './WalletCard/WalletCard';

export const UserWallets = () => {
  const { isConnected, address } = useAccount();
  const user = useUserProfileV2Store((state) => state.user);
  const isMe = useUserProfileV2Store((state) => state.isMe);
  const setModalsState = useModalStore((state) => state.setState);
  const toaster = useToaster();

  const sortedWallet = useMemo(() => {
    if (user && user.wallets) {
      return alphabetical(user.wallets, (w) => w.name);
    }
    return [];
  }, [user]);

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
