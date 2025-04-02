import './style.scss';

import Skeleton from 'react-loading-skeleton';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { Card } from '../../../../shared/defguard-ui/components/Layout/Card/Card';
import { MessageBox } from '../../../../shared/defguard-ui/components/Layout/MessageBox/MessageBox';
import { MessageBoxType } from '../../../../shared/defguard-ui/components/Layout/MessageBox/types';
import { useAppStore } from '../../../../shared/hooks/store/useAppStore';
import { useUserProfileStore } from '../../../../shared/hooks/store/useUserProfileStore';
import { ChangeSelfPasswordModal } from './modals/ChangeSelfPasswordModal/ChangeSelfPasswordModal';
import { ManageWebAuthNKeysModal } from './modals/ManageWebAuthNModal/ManageWebAuthNModal';
import { RecoveryCodesModal } from './modals/RecoveryCodesModal/RecoveryCodesModal';
import { RegisterEmailMFAModal } from './modals/RegisterEmailMFAModal/RegisterEmailMFAModal';
import { RegisterTOTPModal } from './modals/RegisterTOTPModal/RegisterTOTPModal';
import { UserAuthInfoMFA } from './UserAuthInfoMFA';
import { UserAuthInfoPassword } from './UserAuthInfoPassword';

export const UserAuthInfo = () => {
  const { LL } = useI18nContext();
  const userProfile = useUserProfileStore((state) => state.userProfile);
  const ldapInfo = useAppStore((state) => state.appInfo?.ldap_info);
  return (
    <section id="user-auth-info">
      <header>
        <h2>{LL.userPage.userAuthInfo.header()}</h2>
      </header>
      {userProfile && (
        <Card>
          {userProfile.user.ldap_pass_requires_change && ldapInfo?.enabled && (
            <MessageBox
              type={MessageBoxType.WARNING}
              message={
                <p>
                  <p>
                    {LL.userPage.userAuthInfo.password.ldap_change_heading({
                      ldapName: ldapInfo.ad ? 'Active Directory' : 'LDAP',
                    })}
                  </p>
                  {LL.userPage.userAuthInfo.password.ldap_change_message({
                    ldapName: ldapInfo.ad ? 'Active Directory' : 'LDAP',
                  })}
                </p>
              }
              dismissId="user-password-change-dismiss"
            />
          )}
          <UserAuthInfoPassword />
          <UserAuthInfoMFA />
        </Card>
      )}
      {!userProfile && <Skeleton />}
      <ManageWebAuthNKeysModal />
      <RegisterTOTPModal />
      <RecoveryCodesModal />
      <ChangeSelfPasswordModal />
      <RegisterEmailMFAModal />
    </section>
  );
};
