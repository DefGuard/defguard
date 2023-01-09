import './style.scss';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { Card } from '../../../../shared/components/layout/Card/Card';
import { ManageWebAuthNKeysModal } from './modals/ManageWebAuthNModal/ManageWebAuthNModal';
import { RecoveryCodesModal } from './modals/RecoveryCodesModal/RecoveryCodesModal';
import { RegisterTOTPModal } from './modals/RegisterTOTPModal/RegisterTOTPModal';
import { UserAuthInfoMFA } from './UserAuthInfoMFA';
import { UserAuthInfoPassword } from './UserAuthInfoPassword';

export const UserAuthInfo = () => {
  const { LL } = useI18nContext();
  return (
    <section id="user-auth-info">
      <header>
        <h2>{LL.userPage.userAuthInfo.header()}</h2>
      </header>
      <Card>
        <UserAuthInfoPassword />
        <UserAuthInfoMFA />
      </Card>
      <ManageWebAuthNKeysModal />
      <RegisterTOTPModal />
      <RecoveryCodesModal />
    </section>
  );
};
