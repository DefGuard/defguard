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
import { EditButton } from '../../../../shared/components/layout/EditButton/EditButton';
import {
  EditButtonOption,
  EditButtonOptionStyleVariant,
} from '../../../../shared/components/layout/EditButton/EditButtonOption';
import { RowBox } from '../../../../shared/components/layout/RowBox/RowBox';
import { useModalStore } from '../../../../shared/hooks/store/useModalStore';
import { useUserProfileV2Store } from '../../../../shared/hooks/store/useUserProfileV2Store';
import { UserMFAMethod } from '../../../../shared/types';

export const UserAuthInfo = () => {
  const user = useUserProfileV2Store((store) => store.user);
  const editMode = useUserProfileV2Store((store) => store.editMode);
  const setChangePasswordModal = useModalStore(
    (state) => state.setChangePasswordModal
  );

  return (
    <section id="user-auth-info">
      <h2>Password and authentication</h2>
      <Card>
        {editMode && (
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
            {editMode && (
              <EditButton className="edit-mfa">
                <EditButtonOption text="Enable MFA" />
                <EditButtonOption
                  text="Disable MFA"
                  styleVariant={EditButtonOptionStyleVariant.WARNING}
                />
              </EditButton>
            )}
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
          {editMode ? (
            <>
              <RowBox>
                <p>Authenticator code</p>
                <div className="right">
                  <span>STATIC</span>
                  <EditButton>
                    <EditButtonOption text="Edit" />
                    <EditButtonOption
                      text="Disable"
                      styleVariant={EditButtonOptionStyleVariant.WARNING}
                    />
                    <EditButtonOption text="Enable" />
                    <EditButtonOption text="Make default" />
                  </EditButton>
                </div>
              </RowBox>
              <RowBox>
                <p>Security keys</p>
                <div className="right">
                  <span>STATIC</span>
                  <EditButton>
                    <EditButtonOption text="Edit" />
                    <EditButtonOption
                      text="Disable"
                      styleVariant={EditButtonOptionStyleVariant.WARNING}
                    />
                    <EditButtonOption text="Enable" />
                    <EditButtonOption text="Make default" />
                  </EditButton>
                </div>
              </RowBox>
              <RowBox>
                <p>Wallets</p>
                <div className="right">
                  <span>STATIC</span>
                  <EditButton>
                    <EditButtonOption text="Edit" />
                    <EditButtonOption
                      text="Disable"
                      styleVariant={EditButtonOptionStyleVariant.WARNING}
                    />
                    <EditButtonOption text="Enable" />
                    <EditButtonOption text="Make default" />
                  </EditButton>
                </div>
              </RowBox>
            </>
          ) : (
            <>
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
            </>
          )}
        </section>
        <Divider />
        <section className="recovery">
          <header>
            <h3>Recovery options</h3>
          </header>
          {editMode ? (
            <>
              <RowBox>
                <p>Recovery Codes</p>
                <div className="right">
                  <span>Static</span>
                  <EditButton>
                    <EditButtonOption text="Generate recovery codes" />
                  </EditButton>
                </div>
              </RowBox>
            </>
          ) : (
            <>
              <div className="row">
                <p>Recovery codes</p>
                <p className="info">Static</p>
              </div>
            </>
          )}
        </section>
      </Card>
    </section>
  );
};
