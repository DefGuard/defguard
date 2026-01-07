import './style.scss';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { isPresent } from '../../../../shared/defguard-ui/utils/isPresent';
import { useUserProfileStore } from '../../../../shared/hooks/store/useUserProfileStore';
import { AddComponentBox } from '../../shared/components/AddComponentBox/AddComponentBox';
import { useAddAuthorizationKeyModal } from '../../shared/modals/AddAuthenticationKeyModal/useAddAuthorizationKeyModal';
import { RenameAuthenticationKeyModal } from '../../shared/modals/RenameAuthenticationKeyModal/RenameAuthenticationKeyModal';
import { AuthenticationKeyList } from './AuthenticationKeyList/AuthenticationKeyList';
import { DeleteAuthenticationKeyModal } from './DeleteAuthenticationKeyModal/DeleteAuthenticationKeyModal';

export const UserAuthenticationKeys = () => {
  const { LL } = useI18nContext();
  const user = useUserProfileStore((state) => state.userProfile?.user);
  const openAddAuthenticationKeyModal = useAddAuthorizationKeyModal((s) => s.open);

  return (
    <section id="user-authentication-keys">
      <header>
        <h2>{LL.userPage.authenticationKeys.header()}</h2>
      </header>
      <AuthenticationKeyList />
      {isPresent(user) && (
        <AddComponentBox
          data-testid="add-authentication-key-button"
          text={LL.userPage.authenticationKeys.addKey()}
          callback={() => {
            if (user) {
              openAddAuthenticationKeyModal({
                user,
                selectedMode: 'ssh',
              });
            }
          }}
        />
      )}
      <DeleteAuthenticationKeyModal />
      <RenameAuthenticationKeyModal />
    </section>
  );
};
