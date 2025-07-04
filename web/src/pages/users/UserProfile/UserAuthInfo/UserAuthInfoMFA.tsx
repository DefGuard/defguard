import { useMutation, useQueryClient } from '@tanstack/react-query';
import { cloneDeep } from 'lodash-es';
import { useMemo } from 'react';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { ActivityStatus } from '../../../../shared/defguard-ui/components/Layout/ActivityStatus/ActivityStatus';
import { ActivityType } from '../../../../shared/defguard-ui/components/Layout/ActivityStatus/types';
import { EditButton } from '../../../../shared/defguard-ui/components/Layout/EditButton/EditButton';
import { EditButtonOption } from '../../../../shared/defguard-ui/components/Layout/EditButton/EditButtonOption';
import { EditButtonOptionStyleVariant } from '../../../../shared/defguard-ui/components/Layout/EditButton/types';
import { RowBox } from '../../../../shared/defguard-ui/components/Layout/RowBox/RowBox';
import { useAppStore } from '../../../../shared/hooks/store/useAppStore.ts';
import { useAuthStore } from '../../../../shared/hooks/store/useAuthStore.ts';
import { useModalStore } from '../../../../shared/hooks/store/useModalStore';
import { useUserProfileStore } from '../../../../shared/hooks/store/useUserProfileStore';
import useApi from '../../../../shared/hooks/useApi';
import { useToaster } from '../../../../shared/hooks/useToaster';
import { MutationKeys } from '../../../../shared/mutations';
import { QueryKeys } from '../../../../shared/queries';
import { UserMFAMethod } from '../../../../shared/types';
import { useEmailMFAModal } from './modals/RegisterEmailMFAModal/hooks/useEmailMFAModal.tsx';

export const UserAuthInfoMFA = () => {
  const { LL, locale } = useI18nContext();
  const userProfile = useUserProfileStore((store) => store.userProfile);
  const isMe = useUserProfileStore((store) => store.isMe);
  const isAdmin = useAuthStore((state) => state.user?.is_admin);
  const isMeOrAdmin = isMe || isAdmin;
  const editMode = useUserProfileStore((store) => store.editMode);
  const setModalsState = useModalStore((store) => store.setState);
  const smtpEnabled = useAppStore((state) => state.appInfo?.smtp_enabled);
  const openEmailMFAModal = useEmailMFAModal((state) => state.open);
  const queryClient = useQueryClient();

  const refreshUserQueries = () => {
    void queryClient.invalidateQueries({
      queryKey: [QueryKeys.FETCH_USER_PROFILE],
    });
  };

  const {
    user: { editUser, disableUserMfa },
    auth: {
      mfa: {
        totp: { disable: disableTOTP },
        email: { disable: disableEmailMFA },
      },
    },
  } = useApi();

  const mfaWebAuthNEnabled = useMemo(
    () => userProfile?.security_keys && userProfile.security_keys.length > 0,
    [userProfile],
  );

  const toaster = useToaster();

  const { mutate: disableMFA } = useMutation({
    mutationKey: [MutationKeys.DISABLE_MFA],
    mutationFn: disableUserMfa,
    onSuccess: () => {
      refreshUserQueries();
      toaster.success(LL.userPage.userAuthInfo.mfa.messages.mfaDisabled());
    },
    onError: (err) => {
      toaster.error(LL.messages.error());
      console.error(err);
    },
  });

  const { mutate: disableTOTPMutation } = useMutation({
    mutationKey: [MutationKeys.DISABLE_TOTP],
    mutationFn: disableTOTP,
    onSuccess: () => {
      refreshUserQueries();
      toaster.success(LL.userPage.userAuthInfo.mfa.messages.OTPDisabled());
    },
    onError: (err) => {
      toaster.error(LL.messages.error());
      console.error(err);
    },
  });

  const { mutate: disableEmailMFAMutation } = useMutation({
    mutationKey: [MutationKeys.DISABLE_EMAIL_MFA],
    mutationFn: disableEmailMFA,
    onSuccess: () => {
      refreshUserQueries();
      toaster.success(LL.userPage.userAuthInfo.mfa.messages.EmailMFADisabled());
    },
    onError: (err) => {
      toaster.error(LL.messages.error());
      console.error(err);
    },
  });

  const { mutate: editUserMutation } = useMutation({
    mutationKey: [MutationKeys.EDIT_USER],
    mutationFn: editUser,
    onSuccess: () => {
      toaster.success(LL.userPage.userAuthInfo.mfa.messages.changeMFAMethod());
      void queryClient.invalidateQueries({
        queryKey: [QueryKeys.FETCH_USER_PROFILE],
      });
    },
    onError: () => {
      toaster.error(LL.messages.error());
    },
  });

  const changeDefaultMFAMethod = (mfaMethod: UserMFAMethod) => {
    if (userProfile) {
      const userClone = cloneDeep(userProfile.user);
      userClone.mfa_method = mfaMethod;
      editUserMutation({
        username: userProfile.user.username,
        data: userClone,
      });
    }
  };

  const getTOTPInfoText = useMemo(() => {
    if (userProfile?.user.totp_enabled) {
      const res: string[] = [LL.userPage.userAuthInfo.mfa.enabled()];
      if (userProfile?.user.mfa_method === UserMFAMethod.ONE_TIME_PASSWORD) {
        const defaultStr = `(${LL.userPage.userAuthInfo.mfa.default()})`;
        res.push(defaultStr);
      }
      return res.join(' ');
    }
    return LL.userPage.userAuthInfo.mfa.disabled();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [userProfile, locale]);

  const getEmailMFAInfoText = useMemo(() => {
    if (userProfile?.user.email_mfa_enabled) {
      const res: string[] = [LL.userPage.userAuthInfo.mfa.enabled()];
      if (userProfile?.user.mfa_method === UserMFAMethod.EMAIL) {
        const defaultStr = `(${LL.userPage.userAuthInfo.mfa.default()})`;
        res.push(defaultStr);
      }
      return res.join(' ');
    }
    return LL.userPage.userAuthInfo.mfa.disabled();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [userProfile, locale]);

  const getWebAuthNInfoText = useMemo(() => {
    if (userProfile) {
      if (userProfile.security_keys && userProfile.security_keys.length) {
        const res = [
          `${userProfile.security_keys.length} ${
            userProfile.security_keys.length > 1
              ? LL.userPage.userAuthInfo.mfa.securityKey.plural()
              : LL.userPage.userAuthInfo.mfa.securityKey.singular()
          }`,
        ];
        if (userProfile.user.mfa_method === UserMFAMethod.WEB_AUTH_N) {
          res.push(`(${LL.userPage.userAuthInfo.mfa.default()})`);
        }
        return res.join(' ');
      }
    }
    return LL.userPage.userAuthInfo.mfa.disabled();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [userProfile, locale]);

  return (
    <section className="mfa">
      <header>
        {editMode && isMeOrAdmin && (
          <EditButton className="edit-mfa" visible={userProfile?.user.mfa_enabled}>
            <EditButtonOption
              text={LL.userPage.userAuthInfo.mfa.edit.disable()}
              styleVariant={EditButtonOptionStyleVariant.WARNING}
              onClick={() => userProfile && disableMFA(userProfile?.user.username)}
            />
          </EditButton>
        )}
        <h3>{LL.userPage.userAuthInfo.mfa.header()}</h3>
        <span className="status">
          <ActivityStatus
            connectionStatus={
              userProfile?.user.mfa_enabled ? ActivityType.SUCCESS : ActivityType.ERROR
            }
            message={
              userProfile?.user.mfa_enabled
                ? LL.userPage.userAuthInfo.mfa.enabled()
                : LL.userPage.userAuthInfo.mfa.disabled()
            }
            reversed
          />
        </span>
      </header>
      {editMode && isMe ? (
        <>
          <RowBox>
            <p>{LL.userPage.userAuthInfo.mfa.labels.totp()}</p>
            <div className="right">
              <span>{getTOTPInfoText}</span>
              <EditButton data-testid="edit-totp">
                {userProfile?.user.totp_enabled && (
                  <EditButtonOption
                    onClick={() => disableTOTPMutation()}
                    text={LL.userPage.userAuthInfo.mfa.editMode.disable()}
                    styleVariant={EditButtonOptionStyleVariant.WARNING}
                  />
                )}
                {!userProfile?.user.totp_enabled && (
                  <EditButtonOption
                    data-testid="enable-totp-option"
                    text={LL.userPage.userAuthInfo.mfa.editMode.enable()}
                    onClick={() => setModalsState({ registerTOTP: { visible: true } })}
                  />
                )}
                <EditButtonOption
                  disabled={
                    !userProfile?.user.totp_enabled ||
                    userProfile?.user.mfa_method === UserMFAMethod.ONE_TIME_PASSWORD
                  }
                  text={LL.userPage.userAuthInfo.mfa.editMode.makeDefault()}
                  onClick={() => changeDefaultMFAMethod(UserMFAMethod.ONE_TIME_PASSWORD)}
                />
              </EditButton>
            </div>
          </RowBox>
          {smtpEnabled && (
            <RowBox>
              <p>{LL.userPage.userAuthInfo.mfa.labels.email()}</p>
              <div className="right">
                <span>{getEmailMFAInfoText}</span>
                <EditButton data-testid="edit-email-mfa">
                  {userProfile?.user.email_mfa_enabled && (
                    <EditButtonOption
                      onClick={() => disableEmailMFAMutation()}
                      text={LL.userPage.userAuthInfo.mfa.editMode.disable()}
                      styleVariant={EditButtonOptionStyleVariant.WARNING}
                    />
                  )}
                  {!userProfile?.user.email_mfa_enabled && (
                    <EditButtonOption
                      data-testid="enable-email-mfa-option"
                      text={LL.userPage.userAuthInfo.mfa.editMode.enable()}
                      onClick={() => openEmailMFAModal()}
                    />
                  )}
                  <EditButtonOption
                    disabled={
                      !userProfile?.user.email_mfa_enabled ||
                      userProfile?.user.mfa_method === UserMFAMethod.EMAIL
                    }
                    text={LL.userPage.userAuthInfo.mfa.editMode.makeDefault()}
                    onClick={() => changeDefaultMFAMethod(UserMFAMethod.EMAIL)}
                  />
                </EditButton>
              </div>
            </RowBox>
          )}
          <RowBox>
            <p>{LL.userPage.userAuthInfo.mfa.labels.webauth()}</p>
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
                    userProfile?.user.mfa_method === UserMFAMethod.WEB_AUTH_N ||
                    !mfaWebAuthNEnabled
                  }
                  text={LL.userPage.userAuthInfo.mfa.editMode.makeDefault()}
                  onClick={() => changeDefaultMFAMethod(UserMFAMethod.WEB_AUTH_N)}
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
          {smtpEnabled && (
            <div className="row">
              <p>{LL.userPage.userAuthInfo.mfa.labels.email()}</p>
              <p className="info">{getEmailMFAInfoText}</p>
            </div>
          )}
          <div className="row">
            <p>{LL.userPage.userAuthInfo.mfa.labels.webauth()}</p>
            <p className="info">{getWebAuthNInfoText}</p>
          </div>
        </>
      )}
    </section>
  );
};
