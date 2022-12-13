import './style.scss';

import { Card } from '../../../../shared/components/layout/Card/Card';
import { ManageWebAuthNKeysModal } from './modals/ManageWebAuthNModal/ManageWebAuthNModal';
import { RecoveryCodesModal } from './modals/RecoveryCodesModal/RecoveryCodesModal';
import { RegisterTOTPModal } from './modals/RegisterTOTPModal/RegisterTOTPModal';
import { UserAuthInfoMFA } from './UserAuthInfoMFA';
import { UserAuthInfoPassword } from './UserAuthInfoPassword';

export const UserAuthInfo = () => {
  return (
    <section id="user-auth-info">
      <header>
        <h2>Password and authentication</h2>
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
