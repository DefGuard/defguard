import './style.scss';

import Skeleton from 'react-loading-skeleton';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { useAuthStore } from '../../../../shared/hooks/store/useAuthStore';
import { useModalStore } from '../../../../shared/hooks/store/useModalStore';
import { useUserProfileStore } from '../../../../shared/hooks/store/useUserProfileStore';
import { AddComponentBox } from '../../shared/components/AddComponentBox/AddComponentBox';
import { KeyBox } from '../../shared/components/KeyBox/KeyBox';

export const UserYubiKeys = () => {
  const { LL } = useI18nContext();
  const user = useUserProfileStore((state) => state.userProfile?.user);
  const isAdmin = useAuthStore((state) => state.isAdmin);
  const setProvisioningModal = useModalStore((state) => state.setProvisionKeyModal);

  return (
    <section id="user-yubikeys">
      <header>
        <h2>{LL.userPage.yubiKey.header()}</h2>
      </header>
      {!user && (
        <div className="skeletons">
          <Skeleton />
          <Skeleton />
          <Skeleton />
        </div>
      )}
      {(user?.pgp_key || user?.ssh_key) && (
        <div className="keys">
          {user.pgp_key && (
            <KeyBox
              collapsible={false}
              keyValue={user.pgp_key}
              title={LL.userPage.yubiKey.keys.pgp()}
            />
          )}
          {user.ssh_key && (
            <KeyBox
              collapsible={false}
              keyValue={user.ssh_key}
              title={LL.userPage.yubiKey.keys.ssh()}
            />
          )}
        </div>
      )}
      {user && isAdmin && (
        <AddComponentBox
          callback={() => {
            if (user) {
              setProvisioningModal({ visible: true, user: user });
            }
          }}
          text={LL.userPage.yubiKey.provision()}
        />
      )}
    </section>
  );
};
