import './style.scss';

import { useMemo } from 'react';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { EditButton } from '../../../../shared/components/layout/EditButton/EditButton';
import { EditButtonOption } from '../../../../shared/components/layout/EditButton/EditButtonOption';
import { NoLicenseBox } from '../../../../shared/components/layout/NoLicenseBox/NoLicenseBox';
import { useAppStore } from '../../../../shared/hooks/store/useAppStore';
import { useAuthStore } from '../../../../shared/hooks/store/useAuthStore';
import { useModalStore } from '../../../../shared/hooks/store/useModalStore';
import { useUserProfileStore } from '../../../../shared/hooks/store/useUserProfileStore';
import { AddComponentBox } from '../../shared/components/AddComponentBox/AddComponentBox';
import KeyBox from '../../shared/components/KeyBox/KeyBox';
import { KeyDetailsModal } from './modals/KeyDetailsModal/KeyDetailsModal';

export const UserYubiKeys = () => {
  const { LL } = useI18nContext();
  const license = useAppStore((state) => state.license);
  const user = useUserProfileStore((state) => state.user);
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
        <h2>{LL.userPage.yubiKey.header()}</h2>
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
              title={LL.userPage.yubiKey.keys.pgp()}
            />
            <KeyBox
              collapsible={false}
              keyValue={user?.ssh_key}
              title={LL.userPage.yubiKey.keys.ssh()}
            />
          </div>
        ) : null
      ) : (
        <NoLicenseBox>
          <p>
            <strong>{LL.userPage.yubiKey.noLicense.moduleName()}</strong>
          </p>
          <br />
          <p>{LL.userPage.yubiKey.noLicense.line1()}</p>
          <p>{LL.userPage.yubiKey.noLicense.line2()}</p>
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
          text={LL.userPage.yubiKey.provision()}
        />
      )}
      <KeyDetailsModal />
    </section>
  );
};
