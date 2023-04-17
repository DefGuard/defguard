import { useMutation, useQueryClient } from '@tanstack/react-query';
import { cloneDeep, isUndefined } from 'lodash-es';
import { useMemo } from 'react';

import { useI18nContext } from '../../../../i18n/i18n-react';
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
import { useUserProfileStore } from '../../../../shared/hooks/store/useUserProfileStore';
import useApi from '../../../../shared/hooks/useApi';
import { useToaster } from '../../../../shared/hooks/useToaster';
import { MutationKeys } from '../../../../shared/mutations';
import { QueryKeys } from '../../../../shared/queries';
import { UserMFAMethod } from '../../../../shared/types';

export const UserAuthInfoMFA = () => {
  const { LL, locale } = useI18nContext();
  const user = useUserProfileStore((store) => store.user);
  const isMe = useUserProfileStore((store) => store.isMe);
  const editMode = useUserProfileStore((store) => store.editMode);
  const setModalsState = useModalStore((store) => store.setState);
  const queryClient = useQueryClient();

  const refreshUserQueries = () => {
    queryClient.invalidateQueries([QueryKeys.FETCH_USER]);
  };

  const {
    user: { editUser },
    auth: {
      mfa: {
        disable,
        totp: { disable: disableTOTP },
      },
    },
  } = useApi();

  const mfaWebAuthNEnabled = useMemo(
    () => user?.security_keys && user.security_keys.length > 0,
    [user]
  );

  const mfaWeb3Enabled = useMemo(
    () => !isUndefined(user?.wallets.find((w) => w.use_for_mfa === true)),
    [user]
  );

  const toaster = useToaster();

  const { mutate: disableMFA } = useMutation([MutationKeys.DISABLE_MFA], disable, {
    onSuccess: () => {
      refreshUserQueries();
      toaster.success(LL.userPage.userAuthInfo.mfa.messages.mfaDisabled());
    },
    onError: (err) => {
      toaster.error(LL.messages.error());
      console.error(err);
    },
  });

  const { mutate: disableTOTPMutation } = useMutation(
    [MutationKeys.DISABLE_TOTP],
    disableTOTP,
    {
      onSuccess: () => {
        refreshUserQueries();
        toaster.success(LL.userPage.userAuthInfo.mfa.messages.OTPDisabled());
      },
      onError: (err) => {
        toaster.error(LL.messages.error());
        console.error(err);
      },
    }
  );

  const { mutate: editUserMutation } = useMutation([MutationKeys.EDIT_USER], editUser, {
    onSuccess: () => {
      toaster.success(LL.userPage.userAuthInfo.mfa.messages.changeMFAMethod());
      queryClient.invalidateQueries([QueryKeys.FETCH_USER]);
    },
    onError: () => {
      toaster.error(LL.messages.error());
    },
  });

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

  const getTOTPInfoText = useMemo(() => {
    if (user?.totp_enabled) {
      const res: string[] = [LL.userPage.userAuthInfo.mfa.enabled()];
      if (user.mfa_method === UserMFAMethod.ONE_TIME_PASSWORD) {
        const defaultStr = `(${LL.userPage.userAuthInfo.mfa.default()})`;
        res.push(defaultStr);
      }
      return res.join(' ');
    }
    return LL.userPage.userAuthInfo.mfa.disabled();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [user, locale]);

  const getWebAuthNInfoText = useMemo(() => {
    if (user) {
      if (user.security_keys && user.security_keys.length) {
        const res = [
          `${user.security_keys.length} ${
            user.security_keys.length > 1
              ? LL.userPage.userAuthInfo.mfa.securityKey.plural()
              : LL.userPage.userAuthInfo.mfa.securityKey.singular()
          }`,
        ];
        if (user.mfa_method === UserMFAMethod.WEB_AUTH_N) {
          res.push(`(${LL.userPage.userAuthInfo.mfa.default()})`);
        }
        return res.join(' ');
      }
    }
    return LL.userPage.userAuthInfo.mfa.disabled();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [user, locale]);

  const getWalletsInfoText = useMemo(() => {
    if (user) {
      const userAuthorizedWallets = user.wallets.filter((w) => w.use_for_mfa);
      if (userAuthorizedWallets && userAuthorizedWallets.length) {
        const res = [
          `${userAuthorizedWallets.length} ${
            userAuthorizedWallets.length > 1
              ? LL.userPage.userAuthInfo.mfa.wallet.plural()
              : LL.userPage.userAuthInfo.mfa.wallet.singular()
          }`,
        ];
        if (user.mfa_method === UserMFAMethod.WEB3) {
          res.push(`(${LL.userPage.userAuthInfo.mfa.default()})`);
        }
        return res.join(' ');
      }
      return LL.userPage.userAuthInfo.mfa.disabled();
    }
    return '';
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [user, locale]);

  return (
    <section className="mfa">
      <header>
        {editMode && isMe && (
          <EditButton className="edit-mfa" visible={user?.mfa_enabled}>
            <EditButtonOption
              text={LL.userPage.userAuthInfo.mfa.edit.disable()}
              styleVariant={EditButtonOptionStyleVariant.WARNING}
              onClick={() => disableMFA()}
            />
          </EditButton>
        )}
        <h3>{LL.userPage.userAuthInfo.mfa.header()}</h3>
        <span className="status">
          <ActivityStatus
            connectionStatus={
              user?.mfa_enabled ? ActivityType.CONNECTED : ActivityType.ALERT
            }
            customMessage={
              user?.mfa_enabled
                ? LL.userPage.userAuthInfo.mfa.enabled()
                : LL.userPage.userAuthInfo.mfa.disabled()
            }
          />
        </span>
      </header>
      {editMode && isMe ? (
        <>
          <RowBox>
            <p>One time password</p>
            <div className="right">
              <span>{getTOTPInfoText}</span>
              <EditButton>
                {user?.totp_enabled && (
                  <EditButtonOption
                    onClick={() => disableTOTPMutation()}
                    text={LL.userPage.userAuthInfo.mfa.editMode.disable()}
                    styleVariant={EditButtonOptionStyleVariant.WARNING}
                  />
                )}
                {!user?.totp_enabled && (
                  <EditButtonOption
                    text={LL.userPage.userAuthInfo.mfa.editMode.enable()}
                    onClick={() => setModalsState({ registerTOTP: { visible: true } })}
                  />
                )}
                <EditButtonOption
                  disabled={
                    !user?.totp_enabled ||
                    user.mfa_method === UserMFAMethod.ONE_TIME_PASSWORD
                  }
                  text={LL.userPage.userAuthInfo.mfa.editMode.makeDefault()}
                  onClick={() => changeDefaultMFAMethod(UserMFAMethod.ONE_TIME_PASSWORD)}
                />
              </EditButton>
            </div>
          </RowBox>
          <RowBox>
            <p>Security keys</p>
            <div className="right">
              <span>{getWebAuthNInfoText}</span>
              <EditButton>
                <EditButtonOption
                  text={LL.userPage.userAuthInfo.mfa.editMode.webauth.manage()}
                  onClick={() =>
                    setModalsState({
                      manageWebAuthNKeysModal: { visible: true },
                    })
                  }
                />
                <EditButtonOption
                  disabled={
                    user?.mfa_method === UserMFAMethod.WEB_AUTH_N || !mfaWebAuthNEnabled
                  }
                  text={LL.userPage.userAuthInfo.mfa.editMode.makeDefault()}
                  onClick={() => changeDefaultMFAMethod(UserMFAMethod.WEB_AUTH_N)}
                />
              </EditButton>
            </div>
          </RowBox>
          <RowBox>
            <p>Wallets</p>
            <div className="right">
              <span>{getWalletsInfoText}</span>
              <EditButton>
                <EditButtonOption
                  disabled={user?.mfa_method === UserMFAMethod.WEB3 || !mfaWeb3Enabled}
                  text={LL.userPage.userAuthInfo.mfa.editMode.makeDefault()}
                  onClick={() => changeDefaultMFAMethod(UserMFAMethod.WEB3)}
                />
              </EditButton>
            </div>
          </RowBox>
        </>
      ) : (
        <>
          <div className="row">
            <p>{LL.userPage.userAuthInfo.mfa.labels.totp()}</p>
            <p className="info">{getTOTPInfoText}</p>
          </div>
          <div className="row">
            <p>{LL.userPage.userAuthInfo.mfa.labels.webauth()}</p>
            <p className="info">{getWebAuthNInfoText}</p>
          </div>
          <div className="row">
            <p>{LL.userPage.userAuthInfo.mfa.labels.wallets()}</p>
            <p className="info">{getWalletsInfoText}</p>
          </div>
        </>
      )}
    </section>
  );
};
