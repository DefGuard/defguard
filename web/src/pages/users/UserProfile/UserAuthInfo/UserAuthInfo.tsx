import './style.scss';

import Skeleton from 'react-loading-skeleton';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { useUserProfileStore } from '../../../../shared/hooks/store/useUserProfileStore';
import { ChangeSelfPasswordModal } from './modals/ChangeSelfPasswordModal/ChangeSelfPasswordModal';
import { ManageWebAuthNKeysModal } from './modals/ManageWebAuthNModal/ManageWebAuthNModal';
import { RecoveryCodesModal } from './modals/RecoveryCodesModal/RecoveryCodesModal';
import { RegisterTOTPModal } from './modals/RegisterTOTPModal/RegisterTOTPModal';
import { UserAuthInfoMFA } from './UserAuthInfoMFA';
import { UserAuthInfoPassword } from './UserAuthInfoPassword';
import { Card } from '../../../../shared/defguard-ui/components/Layout/Card/Card';

export const UserAuthInfo = () => {
  const { LL } = useI18nContext();
  const userProfile = useUserProfileStore((state) => state.userProfile);
  return (
    <section id="user-auth-info">
      <header>
        <h2>{LL.userPage.userAuthInfo.header()}</h2>
      </header>
      {userProfile && (
        <Card>
          <UserAuthInfoPassword />
          <UserAuthInfoMFA />
        </Card>
      )}
      {!userProfile && <Skeleton />}
      <ManageWebAuthNKeysModal />
      <RegisterTOTPModal />
      <RecoveryCodesModal />
      <ChangeSelfPasswordModal />
    </section>
  );
};
