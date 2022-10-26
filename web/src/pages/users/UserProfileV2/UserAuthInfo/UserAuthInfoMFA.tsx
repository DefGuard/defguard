import {
  ActivityStatus,
  ActivityType,
} from '../../../../shared/components/layout/ActivityStatus/ActivityStatus';
import { EditButton } from '../../../../shared/components/layout/EditButton/EditButton';
import {
  EditButtonOption,
  EditButtonOptionStyleVariant,
} from '../../../../shared/components/layout/EditButton/EditButtonOption';
import { RowBox } from '../../../../shared/components/layout/RowBox/RowBox';
import { useModalStore } from '../../../../shared/hooks/store/useModalStore';
import { useUserProfileV2Store } from '../../../../shared/hooks/store/useUserProfileV2Store';
import { UserMFAMethod } from '../../../../shared/types';

export const UserAuthInfoMFA = () => {
  const user = useUserProfileV2Store((store) => store.user);
  const isMe = useUserProfileV2Store((store) => store.isMe);
  const editMode = useUserProfileV2Store((store) => store.editMode);
  const setModalsState = useModalStore((store) => store.setState);

  return (
    <section className="mfa">
      <header>
        {editMode && isMe && (
          <EditButton className="edit-mfa">
            {user?.mfa_enabled ? (
              <EditButtonOption
                text="Disable MFA"
                styleVariant={EditButtonOptionStyleVariant.WARNING}
              />
            ) : (
              <EditButtonOption text="Enable MFA" />
            )}
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
              user?.mfa_method !== UserMFAMethod.NONE ? 'Enable' : 'Disabled'
            }
          />
        </span>
      </header>
      {editMode && isMe ? (
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
                <EditButtonOption
                  text="Manage security keys"
                  onClick={() =>
                    setModalsState({
                      manageWebAuthNKeysModal: { visible: true },
                    })
                  }
                />
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
  );
};
