import './style.scss';

import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { useUserProfileStore } from '../../../../shared/hooks/store/useUserProfileStore';
import { AddComponentBox } from '../../shared/components/AddComponentBox/AddComponentBox';
import { useAddAuthorizationKeyModal } from '../../shared/modals/AddAuthenticationKeyModal/useAddAuthorizationKeyModal';
import { DeleteAuthenticationKeyModal } from '../../shared/modals/DeleteAuthenticationKeyModal/DeleteAuthenticationKeyModal';
import { RenameAuthenticationKeyModal } from '../../shared/modals/RenameAuthenticationKeyModal/RenameAuthenticationKeyModal';
import { AuthenticationKeyList } from './AuthenticationKeyList/AuthenticationKeyList';

export const UserAuthenticationKeys = () => {
  const { LL } = useI18nContext();
  const user = useUserProfileStore((state) => state.userProfile?.user);
  const openAddAuthenticationKeyModal = useAddAuthorizationKeyModal(
    (s) => s.open,
    shallow,
  );

  return (
    <section id="user-authentication-keys">
      <header>
        <h2>{LL.userPage.authenticationKeys.header()}</h2>
      </header>
      <AuthenticationKeyList />
      {user && (
        <AddComponentBox
          data-testid="add-authentication-key-button"
          callback={() => {
            if (user) {
              openAddAuthenticationKeyModal({
                user,
                selectedMode: 'yubikey',
              });
            }
          }}
          text={LL.userPage.authenticationKeys.addKey()}
        />
      )}
      <DeleteAuthenticationKeyModal />
      <RenameAuthenticationKeyModal />
    </section>
  );
};
