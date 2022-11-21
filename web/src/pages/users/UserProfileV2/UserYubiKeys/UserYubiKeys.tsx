import './style.scss';

import { useMemo } from 'react';

import { EditButton } from '../../../../shared/components/layout/EditButton/EditButton';
import { EditButtonOption } from '../../../../shared/components/layout/EditButton/EditButtonOption';
import { NoLicenseBox } from '../../../../shared/components/layout/NoLicenseBox/NoLicenseBox';
import { useAppStore } from '../../../../shared/hooks/store/useAppStore';
import { useAuthStore } from '../../../../shared/hooks/store/useAuthStore';
import { useModalStore } from '../../../../shared/hooks/store/useModalStore';
import { useUserProfileV2Store } from '../../../../shared/hooks/store/useUserProfileV2Store';
import { AddComponentBox } from '../../shared/components/AddComponentBox/AddComponentBox';
import KeyBox from '../../shared/components/KeyBox/KeyBox';

export const UserYubiKeys = () => {
  const license = useAppStore((state) => state.license);
  const user = useUserProfileV2Store((state) => state.user);
  const isAdmin = useAuthStore((state) => state.isAdmin);

  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const enableEdit = useMemo(() => {
    if (user && isAdmin) {
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
        {enableEdit && (
          <EditButton>
            <EditButtonOption text="Provision new keys" />
          </EditButton>
        )}
      </header>
      {license && license.enterprise ? (
        user?.pgp_key || user?.ssh_key ? (
          <div className="keys">
            <KeyBox
              collapsible={false}
              keyValue={user?.pgp_key}
              title="PGP key"
            />
            <KeyBox
              collapsible={false}
              keyValue={user?.ssh_key}
              title="SSH key"
            />
          </div>
        ) : null
      ) : (
        <NoLicenseBox>
          <p>
            <strong>YubiKey module</strong>
          </p>
          <br />
          <p>This is enterprise module for YubiKey</p>
          <p>management and provisioning.</p>
        </NoLicenseBox>
      )}
      {isAdmin && !enableEdit && (
        <AddComponentBox
          disabled={!license?.enterprise}
          callback={() => {
            if (user) {
              setProvisioningModal({ visible: true, user: user });
            }
          }}
          text="Provision a Yubikey"
        />
      )}
    </section>
  );
};
