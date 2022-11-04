import './style.scss';

import { Card } from '../../../../shared/components/layout/Card/Card';
import Divider from '../../../../shared/components/layout/Divider/Divider';
import { ManageWebAuthNKeysModal } from './modals/ManageWebAuthNModal/ManageWebAuthNModal';
import { RecoveryCodesModal } from './modals/RecoveryCodesModal/RecoveryCodesModal';
import { RegisterTOTPModal } from './modals/RegisterTOTPModal/RegisterTOTPModal';
import { UserAuthInfoMFA } from './UserAuthInfoMFA';
import { UserAuthInfoPassword } from './UserAuthInfoPassword';
import { UserAuthInfoRecovery } from './UserAuthInfoRecovery';

export const UserAuthInfo = () => {
  return (
    <section id="user-auth-info">
      <h2>Password and authentication</h2>
      <Card>
        <UserAuthInfoPassword />
        <UserAuthInfoMFA />
        <Divider />
        <UserAuthInfoRecovery />
      </Card>
      <ManageWebAuthNKeysModal />
      <RegisterTOTPModal />
      <RecoveryCodesModal />
    </section>
  );
};
