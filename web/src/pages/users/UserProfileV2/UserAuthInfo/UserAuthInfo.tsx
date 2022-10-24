import './style.scss';

import {
  ActivityStatus,
  ActivityType,
} from '../../../../shared/components/layout/ActivityStatus/ActivityStatus';
import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../shared/components/layout/Button/Button';
import { Card } from '../../../../shared/components/layout/Card/Card';
import Divider from '../../../../shared/components/layout/Divider/Divider';
import { useAuthStore } from '../../../../shared/hooks/store/useAuthStore';
import { useModalStore } from '../../../../shared/hooks/store/useModalStore';
import { useUserProfileV2Store } from '../../../../shared/hooks/store/useUserProfileV2Store';
import { UserMFAMethod } from '../../../../shared/types';

export const UserAuthInfo = () => {
  const user = useUserProfileV2Store((store) => store.user);
  const editMode = useUserProfileV2Store((store) => store.editMode);
  const isAdmin = useAuthStore((state) => state.isAdmin);
  const setChangePasswordModal = useModalStore(
    (state) => state.setChangePasswordModal
  );

  return (
    <section id="user-auth-info">
      <h2>Password and authentication</h2>
      <Card>
        {editMode && isAdmin && (
          <>
            <section className="password">
              <header>
                <h3>Password settings</h3>
              </header>
              <div className="row">
                <Button
                  size={ButtonSize.SMALL}
                  styleVariant={ButtonStyleVariant.STANDARD}
                  text="Change password"
                  onClick={() => {
                    if (user) {
                      setChangePasswordModal({
                        visible: true,
                        user: user,
                      });
                    }
                  }}
                />
              </div>
            </section>
            <Divider />
          </>
        )}
        <section className="two-factor">
          <header>
            <h3>Two-factor methods</h3>
            <span className="status">
              <ActivityStatus
                connectionStatus={
                  user?.mfa_method !== UserMFAMethod.NONE
                    ? ActivityType.CONNECTED
                    : ActivityType.ALERT
                }
                customMessage={
                  user?.mfa_method !== UserMFAMethod.NONE
                    ? 'Enable'
                    : 'Disabled'
                }
              />
            </span>
          </header>
          <div className="row">
            <p>Authentication method</p>
            <p className="info">{user?.mfa_method.valueOf() || 'None'}</p>
          </div>
          <div className="row">
            <p>Security keys</p>
            <p className="info">
              {user && user.security_keys && user.security_keys.length
                ? `${user.security_keys.length} security keys`
                : 'No keys'}
            </p>
          </div>
          <div className="row">
            <p>Wallets</p>
            <p className="info">
              {user && user.wallets.length
                ? user?.wallets.map((w) => w.name)
                : 'No wallets'}
            </p>
          </div>
        </section>
        <Divider />
        <section className="recovery">
          <header>
            <h3>Recovery options</h3>
          </header>
          <div className="row">
            <p>Recovery codes</p>
            <p className="info">Static</p>
          </div>
        </section>
      </Card>
    </section>
  );
};
