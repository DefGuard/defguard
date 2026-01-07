import './style.scss';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { isPresent } from '../../../../shared/defguard-ui/utils/isPresent';
import { useUserProfileStore } from '../../../../shared/hooks/store/useUserProfileStore';
import { AddComponentBox } from '../../shared/components/AddComponentBox/AddComponentBox';
import { useAddApiTokenModal } from '../../shared/modals/AddApiTokenModal/useAddApiTokenModal';
import { RenameApiTokenModal } from '../../shared/modals/RenameApiTokenModal/RenameApiTokenModal';
import { ApiTokenList } from './ApiTokenList/ApiTokenList';
import { DeleteApiTokenModal } from './DeleteApiTokenModal/DeleteApiTokenModal';

export const UserApiTokens = () => {
  const { LL } = useI18nContext();
  const user = useUserProfileStore((state) => state.userProfile?.user);
  const openAddApiTokenModal = useAddApiTokenModal((s) => s.open);

  return (
    <section id="user-api-tokens">
      <header>
        <h2>{LL.userPage.apiTokens.header()}</h2>
      </header>
      <ApiTokenList />
      {isPresent(user) && (
        <AddComponentBox
          data-testid="add-api-token-button"
          text={LL.userPage.apiTokens.addToken()}
          callback={() => {
            if (user) {
              openAddApiTokenModal({
                user,
              });
            }
          }}
        />
      )}
      <DeleteApiTokenModal />
      <RenameApiTokenModal />
    </section>
  );
};
