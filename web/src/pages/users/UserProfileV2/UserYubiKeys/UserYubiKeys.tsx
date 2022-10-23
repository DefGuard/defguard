import './style.scss';

import { useMemo } from 'react';

import { useModalStore } from '../../../../shared/hooks/store/useModalStore';
import { useUserProfileV2Store } from '../../../../shared/hooks/store/useUserProfileV2Store';
import AddComponentBox from '../../shared/components/AddComponentBox/AddComponentBox';
import KeyBox from '../../shared/components/KeyBox/KeyBox';

export const UserYubiKeys = () => {
  const user = useUserProfileV2Store((state) => state.user);

  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const enableEdit = useMemo(() => {
    if (user) {
      if (user.pgp_key && user.ssh_key) {
        if (user.ssh_key !== '-' && user.pgp_key !== '-') {
          return true;
        }
      }
    }
    return false;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [user?.pgp_key, user?.ssh_key]);

  const setProvisioningModal = useModalStore(
    (state) => state.setProvisionKeyModal
  );

  return (
    <section id="user-yubikeys">
      <header>
        <h2>User YubiKey</h2>
      </header>
      <div className="keys">
        <KeyBox keyValue={user?.pgp_key} title="PGP key" />
        <KeyBox keyValue={user?.ssh_key} title="SSH key" />
      </div>
      <AddComponentBox
        callback={() => {
          if (user) {
            setProvisioningModal({ visible: true, user: user });
          }
        }}
        text="Provision a Yubikey"
      />
    </section>
  );
};
