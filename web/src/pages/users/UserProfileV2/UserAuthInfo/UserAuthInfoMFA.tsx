import { useMutation, useQueryClient } from '@tanstack/react-query';
import { cloneDeep, isUndefined } from 'lodash-es';
import { useMemo } from 'react';

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
import useApi from '../../../../shared/hooks/useApi';
import { MutationKeys } from '../../../../shared/mutations';
import { QueryKeys } from '../../../../shared/queries';
import { UserMFAMethod } from '../../../../shared/types';
import { toaster } from '../../../../shared/utils/toaster';

export const UserAuthInfoMFA = () => {
  const user = useUserProfileV2Store((store) => store.user);
  const isMe = useUserProfileV2Store((store) => store.isMe);
  const editMode = useUserProfileV2Store((store) => store.editMode);
  const setModalsState = useModalStore((store) => store.setState);
  const queryClient = useQueryClient();

  const refreshUserQueries = () => {
    queryClient.invalidateQueries([QueryKeys.FETCH_USER]);
  };

  const {
    user: { editUser },
    auth: {
      mfa: {
        enable,
        disable,
        totp: { disable: disableTOTP },
      },
    },
  } = useApi();

  const { mutate: enableMFA, isLoading: enableMFALoading } = useMutation(
    [MutationKeys.ENABLE_MFA],
    enable,
    {
      onSuccess: () => {
        refreshUserQueries();
        toaster.success('MFA enabled');
      },
      onError: (err) => {
        console.error(err);
        toaster.error('Enabling MFA failed.');
      },
    }
  );

  const mfaWebAuthNEnabled = useMemo(
    () => user?.security_keys && user.security_keys.length > 0,
    [user]
  );

  const mfaWeb3Enabled = useMemo(
    () => !isUndefined(user?.wallets.find((w) => w.use_for_mfa === true)),
    [user]
  );

  const { mutate: disableMFA } = useMutation(
    [MutationKeys.DISABLE_MFA],
    disable,
    {
      onSuccess: () => {
        refreshUserQueries();
        toaster.success('MFA disabled');
      },
      onError: (err) => {
        console.error(err);
        toaster.error('Disabling MFA failed');
      },
    }
  );

  const { mutate: disableTOTPMutation } = useMutation(
    [MutationKeys.DISABLE_TOTP],
    disableTOTP,
    {
      onSuccess: () => {
        refreshUserQueries();
        toaster.success('One time password disabled');
      },
      onError: (err) => {
        console.error(err);
        toaster.error('Disabling one time password failed');
      },
    }
  );

  const { mutate: editUserMutation } = useMutation(
    [MutationKeys.EDIT_USER],
    editUser,
    {
      onSuccess: () => {
        toaster.success('User updated');
        queryClient.invalidateQueries([QueryKeys.FETCH_USER]);
      },
      onError: () => {
        toaster.error('User update failed');
      },
    }
  );

  const changeDefaultMFAMethod = (mfaMethod: UserMFAMethod) => {
    if (user) {
      const userClone = cloneDeep(user);
      userClone.mfa_method = mfaMethod;
      editUserMutation({
        username: user.username,
        data: userClone,
      });
    }
  };

  return (
    <section className="mfa">
      <header>
        {editMode && isMe && (
          <EditButton className="edit-mfa">
            {user?.mfa_enabled ? (
              <EditButtonOption
                text="Disable MFA"
                styleVariant={EditButtonOptionStyleVariant.WARNING}
                onClick={() => disableMFA()}
              />
            ) : (
              <EditButtonOption
                text="Enable MFA"
                onClick={() => enableMFA()}
                disabled={enableMFALoading}
              />
            )}
          </EditButton>
        )}
        <h3>Two-factor methods</h3>
        <span className="status">
          <ActivityStatus
            connectionStatus={
              user?.mfa_enabled ? ActivityType.CONNECTED : ActivityType.ALERT
            }
            customMessage={user?.mfa_enabled ? 'Enabled' : 'Disabled'}
          />
        </span>
      </header>
      {editMode && isMe ? (
        <>
          <RowBox>
            <p>One time password</p>
            <div className="right">
              <span>STATIC</span>
              <EditButton>
                {user?.totp_enabled && (
                  <EditButtonOption
                    onClick={() => disableTOTPMutation()}
                    text="Disable"
                    styleVariant={EditButtonOptionStyleVariant.WARNING}
                  />
                )}
                {!user?.totp_enabled && (
                  <EditButtonOption
                    text="Enable"
                    onClick={() =>
                      setModalsState({ registerTOTP: { visible: true } })
                    }
                  />
                )}
                {user?.mfa_method !== UserMFAMethod.ONE_TIME_PASSWORD &&
                  user?.totp_enabled && (
                    <EditButtonOption
                      text="Make default"
                      onClick={() =>
                        changeDefaultMFAMethod(UserMFAMethod.ONE_TIME_PASSWORD)
                      }
                    />
                  )}
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
                {user?.mfa_method !== UserMFAMethod.WEB_AUTH_N &&
                  mfaWebAuthNEnabled && (
                    <EditButtonOption
                      text="Make default"
                      onClick={() =>
                        changeDefaultMFAMethod(UserMFAMethod.WEB_AUTH_N)
                      }
                    />
                  )}
              </EditButton>
            </div>
          </RowBox>
          <RowBox>
            <p>Wallets</p>
            <div className="right">
              <span>STATIC</span>
              <EditButton
                visible={
                  user?.mfa_method !== UserMFAMethod.WEB3 && mfaWeb3Enabled
                }
              >
                <EditButtonOption
                  text="Make default"
                  onClick={() => changeDefaultMFAMethod(UserMFAMethod.WEB3)}
                />
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
