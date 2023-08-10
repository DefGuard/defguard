import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { Button } from '../../../../shared/components/layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../shared/components/layout/Button/types';
import { Divider } from '../../../../shared/components/layout/Divider/Divider';
import { useModalStore } from '../../../../shared/hooks/store/useModalStore';
import { useUserProfileStore } from '../../../../shared/hooks/store/useUserProfileStore';
import { useChangeSelfPasswordModal } from './modals/ChangeSelfPasswordModal/hooks/useChangeSelfPasswordModal';

export const UserAuthInfoPassword = () => {
  const { LL } = useI18nContext();
  const [user, isMe] = useUserProfileStore(
    (store) => [store.userProfile?.user, store.isMe],
    shallow
  );
  const editMode = useUserProfileStore((store) => store.editMode);
  const setChangePasswordModal = useModalStore((state) => state.setChangePasswordModal);
  const openSelfPasswordModal = useChangeSelfPasswordModal((state) => state.open);

  if (!editMode) return null;
  return (
    <>
      <section className="password">
        <header>
          <h3>{LL.userPage.userAuthInfo.password.header()}</h3>
        </header>
        <div className="row">
          <Button
            size={ButtonSize.SMALL}
            styleVariant={ButtonStyleVariant.STANDARD}
            text={LL.userPage.userAuthInfo.password.changePassword()}
            onClick={() => {
              if (user) {
                if (isMe) {
                  openSelfPasswordModal();
                } else {
                  setChangePasswordModal({
                    visible: true,
                    user: user,
                  });
                }
              }
            }}
            data-testid="button-change-password"
          />
        </div>
      </section>
      <Divider />
    </>
  );
};
