import './style.scss';

import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { useUserProfileStore } from '../../../../shared/hooks/store/useUserProfileStore';
import { AddComponentBox } from '../../shared/components/AddComponentBox/AddComponentBox';
import { useAddApiTokenModal } from '../../shared/modals/AddApiTokenModal/useAddApiTokenModal';
import { RenameApiTokenModal } from '../../shared/modals/RenameApiTokenModal/RenameApiTokenModal';
import { ApiTokenList } from './ApiTokenList/ApiTokenList';
import { DeleteApiTokenModal } from './DeleteApiTokenModal/DeleteApiTokenModal';

export const UserApiTokens = () => {
  const { LL } = useI18nContext();
  const user = useUserProfileStore((state) => state.userProfile?.user);
  const openAddApiTokenModal = useAddApiTokenModal((s) => s.open, shallow);

  return (
    <section id="user-api-tokens">
      <header>
        <h2>{LL.userPage.apiTokens.header()}</h2>
      </header>
      <ApiTokenList />
      {user && (
        <AddComponentBox
          data-testid="add-api-token-button"
          callback={() => {
            if (user) {
              openAddApiTokenModal({
                user,
              });
            }
          }}
          text={LL.userPage.apiTokens.addToken()}
        />
      )}
      <DeleteApiTokenModal />
      <RenameApiTokenModal />
    </section>
  );
};
