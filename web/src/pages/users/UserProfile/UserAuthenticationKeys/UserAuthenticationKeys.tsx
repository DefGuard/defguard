import './style.scss';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { useAuthStore } from '../../../../shared/hooks/store/useAuthStore';
import { useModalStore } from '../../../../shared/hooks/store/useModalStore';
import { useUserProfileStore } from '../../../../shared/hooks/store/useUserProfileStore';
import { AddComponentBox } from '../../shared/components/AddComponentBox/AddComponentBox';
import { AddAuthenticationKeyModal } from '../../shared/modals/AddAuthenticationKeyModal/AddAuthenticationKeyModal';
import { DeleteAuthenticationKeyModal } from '../../shared/modals/DeleteAuthenticationKeyModal/DeleteAuthenticationKeyModal';
import { AuthenticationKeyList } from './AuthenticationKeyList/AuthenticationKeyList';

export const UserAuthenticationKeys = () => {
  const { LL } = useI18nContext();
  const user = useUserProfileStore((state) => state.userProfile?.user);
  const isAdmin = useAuthStore((state) => state.isAdmin);
  const setAddAuthenticationKeyModal = useModalStore(
    (state) => state.setAddAuthenticationKeyModal,
  );

  return (
    <section id="user-yubikeys">
      <header>
        <h2>{LL.userPage.authenticationKeys.header()}</h2>
      </header>
      <AuthenticationKeyList />
      {user && isAdmin && (
        <AddComponentBox
          data-testid="add-authentication-key-button"
          callback={() => {
            if (user) {
              setAddAuthenticationKeyModal({ visible: true, user: user });
            }
          }}
          text={LL.userPage.authenticationKeys.addKey()}
        />
      )}
      <AddAuthenticationKeyModal />
      <DeleteAuthenticationKeyModal />
    </section>
  );
};
