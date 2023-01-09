import './style.scss';

import { isUndefined } from 'lodash-es';
import { alphabetical } from 'radash';
import { useMemo } from 'react';
import { useAccount } from 'wagmi';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { useModalStore } from '../../../../shared/hooks/store/useModalStore';
import { useUserProfileStore } from '../../../../shared/hooks/store/useUserProfileStore';
import { useToaster } from '../../../../shared/hooks/useToaster';
import { AddComponentBox } from '../../shared/components/AddComponentBox/AddComponentBox';
import { AddWalletModal } from './AddWalletModal/AddWalletModal';
import { WalletCard } from './WalletCard/WalletCard';

export const UserWallets = () => {
  const { LL } = useI18nContext();
  const { isConnected, address } = useAccount();
  const user = useUserProfileStore((state) => state.user);
  const isMe = useUserProfileStore((state) => state.isMe);
  const setModalsState = useModalStore((state) => state.setState);
  const toaster = useToaster();

  const sortedWallet = useMemo(() => {
    if (user && user.wallets) {
      return alphabetical(user.wallets, (w) => w.name);
    }
    return [];
  }, [user]);

  const handleAddWallet = () => {
    if (
      isConnected &&
      user &&
      !isUndefined(user.wallets.find((w) => w.address === address))
    ) {
      toaster.warning(
        LL.userPage.wallets.messages.duplicate.primary(),
        LL.userPage.wallets.messages.duplicate.sub()
      );
    } else {
      setModalsState({ addWalletModal: { visible: true } });
    }
  };

  return (
    <section id="user-wallets">
      <header>
        <h2>{LL.userPage.wallets.header()}</h2>
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
        callback={() => {
          if (!isConnected) {
            setModalsState({
              connectWalletModal: { visible: true, onConnect: handleAddWallet },
            });
          } else {
            handleAddWallet();
          }
        }}
        text={LL.userPage.wallets.addWallet()}
      />
      <AddWalletModal />
    </section>
  );
};
