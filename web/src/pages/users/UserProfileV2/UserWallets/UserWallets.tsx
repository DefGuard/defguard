import './style.scss';

import { alphabetical } from 'radash';
import { useMemo } from 'react';

import { useModalStore } from '../../../../shared/hooks/store/useModalStore';
import { useUserProfileV2Store } from '../../../../shared/hooks/store/useUserProfileV2Store';
import AddComponentBox from '../../shared/components/AddComponentBox/AddComponentBox';
import { WalletCard } from './WalletCard/WalletCard';

export const UserWallets = () => {
  const user = useUserProfileV2Store((state) => state.user);
  const setChangeWalletModalState = useModalStore(
    (state) => state.setChangeWalletModal
  );

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
            <WalletCard key={wallet.address} wallet={wallet} />
          ))}
        </div>
      )}
      <AddComponentBox
        callback={() =>
          setChangeWalletModalState({ visible: true, user: user })
        }
        text="Add new wallet"
      />
    </section>
  );
};
