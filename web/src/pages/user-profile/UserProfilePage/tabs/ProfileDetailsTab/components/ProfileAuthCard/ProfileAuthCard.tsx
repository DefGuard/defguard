import './style.scss';
import { type QueryKey, useMutation } from '@tanstack/react-query';
import { useMemo } from 'react';
import { m } from '../../../../../../../paraglide/messages';
import api from '../../../../../../../shared/api/api';
import {
  UserMfaMethod,
  type UserMfaMethodValue,
} from '../../../../../../../shared/api/types';
import { Badge } from '../../../../../../../shared/defguard-ui/components/Badge/Badge';
import { Divider } from '../../../../../../../shared/defguard-ui/components/Divider/Divider';
import { IconButtonMenu } from '../../../../../../../shared/defguard-ui/components/IconButtonMenu/IconButtonMenu';
import type {
  MenuItemProps,
  MenuItemsGroup,
} from '../../../../../../../shared/defguard-ui/components/Menu/types';
import { Snackbar } from '../../../../../../../shared/defguard-ui/providers/snackbar/snackbar';
import { isPresent } from '../../../../../../../shared/defguard-ui/utils/isPresent';
import { openModal } from '../../../../../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../../../../../shared/hooks/modalControls/modalTypes';
import { useApp } from '../../../../../../../shared/hooks/useApp';
import { useAuth } from '../../../../../../../shared/hooks/useAuth';
import { ProfileCard } from '../../../../components/ProfileCard/ProfileCard';
import { useUserProfile } from '../../../../hooks/useUserProfilePage';
import { BiometricDevices } from './components/BiometricDevices/BiometricDevices';
import { FactorRow } from './components/FactorRow/FactorRow';
import { WebauthnRow } from './components/WebauthnRow/WebauthnRow';

const toMenuGroup = (items: MenuItemProps[]): MenuItemsGroup | null =>
  items.length > 0 ? { items } : null;

const makeDefaultItem = (onClick: () => void): MenuItemProps => ({
  icon: 'online',
  text: m.profile_auth_card_make_default(),
  onClick,
});

export const ProfileAuthCard = () => {
  const securityKeys = useUserProfile((s) => s.security_keys);
  const user = useUserProfile((s) => s.user);
  const authUsername = useAuth((s) => s.user?.username as string);
  const isCurrentUserAdmin = useAuth((s) => s.isAdmin);
  const smtpEnabled = useApp((s) => s.appInfo.smtp_enabled);

  const isSelf = user.username === authUsername;
  const hasPasskeys = securityKeys.length > 0;

  const invalidateAfterMfaChange = useMemo(() => {
    const invalidate: QueryKey[] = [
      ['session-info'],
      ...(isSelf ? [['me']] : []),
      ['user', user.username],
      ['activity-log'],
    ];
    return { invalidate };
  }, [isSelf, user.username]);

  const { mutate: mutateSetDefaultMfa } = useMutation({
    mutationFn: (method: UserMfaMethodValue) =>
      api.user.editUser({
        username: user.username,
        body: { ...user, mfa_method: method },
      }),
    meta: invalidateAfterMfaChange,
  });

  const { mutate: mutateEnableMfa } = useMutation({
    mutationFn: api.auth.mfa.enable,
    meta: invalidateAfterMfaChange,
  });

  const emailMenuItems = useMemo(() => {
    const items: MenuItemProps[] = [];
    if (!user.email_mfa_enabled && isSelf) {
      items.push({
        testId: 'enable-email',
        text: m.controls_enable(),
        icon: 'check-circle',
        onClick: () => openModal('emailMfaSetup'),
      });
    }
    if (user.email_mfa_enabled) {
      if (isSelf && user.mfa_method !== UserMfaMethod.Email) {
        items.push(makeDefaultItem(() => mutateSetDefaultMfa(UserMfaMethod.Email)));
      }
      items.push({
        text: m.controls_disable(),
        variant: 'danger',
        icon: 'cancel',
        onClick: () =>
          openModal(ModalName.ConfirmAction, {
            title: m.modal_disable_mfa_email_title(),
            contentMd: m.modal_disable_mfa_email_content(),
            actionPromise: () => api.user.mfa.email.disable(user.username),
            invalidateKeys: invalidateAfterMfaChange.invalidate,
            submitProps: { text: m.controls_disable(), variant: 'critical' },
            onSuccess: () => Snackbar.default(m.modal_disable_mfa_email_success()),
            onError: () => Snackbar.error(m.modal_disable_mfa_email_error()),
          }),
      });
    }
    return toMenuGroup(items);
  }, [
    user.email_mfa_enabled,
    user.mfa_method,
    user.username,
    isSelf,
    mutateSetDefaultMfa,
    invalidateAfterMfaChange,
  ]);

  const mfaMenuItems = useMemo((): MenuItemsGroup[] | null => {
    const hasConfiguredMfa = user.email_mfa_enabled || user.totp_enabled || hasPasskeys;
    if (!hasConfiguredMfa) return null;
    // factors exist but the recovery codes modal was never confirmed
    if (!user.mfa_enabled) {
      return [
        {
          items: [
            {
              text: m.controls_enable(),
              icon: 'check-circle',
              onClick: mutateEnableMfa,
            },
          ],
        },
      ];
    }
    return [
      {
        items: [
          {
            icon: 'disabled',
            variant: 'danger',
            text: m.profile_auth_card_2fa_controls_disable_all(),
            onClick: () =>
              openModal(ModalName.ConfirmAction, {
                title: m.modal_disable_mfa_all_title(),
                contentMd: m.modal_disable_mfa_all_content(),
                actionPromise: () =>
                  isSelf ? api.auth.mfa.disable() : api.user.disableMfa(user.username),
                invalidateKeys: invalidateAfterMfaChange.invalidate,
                submitProps: { text: m.controls_disable(), variant: 'critical' },
                onSuccess: () => Snackbar.default(m.modal_disable_mfa_all_success()),
                onError: () => Snackbar.error(m.modal_disable_mfa_all_error()),
              }),
          },
        ],
      },
    ];
  }, [
    user.email_mfa_enabled,
    user.mfa_enabled,
    user.totp_enabled,
    user.username,
    hasPasskeys,
    isSelf,
    mutateEnableMfa,
    invalidateAfterMfaChange,
  ]);

  const webauthnMenuItems = useMemo(() => {
    const items: MenuItemProps[] = [];
    if (isSelf) {
      items.push({
        text: m.profile_auth_card_add_passkey(),
        icon: 'plus-circle',
        testId: 'add-passkey',
        onClick: () => openModal(ModalName.WebauthnSetup),
      });
    }
    if (hasPasskeys) {
      if (isSelf && user.mfa_method !== UserMfaMethod.Webauthn) {
        items.push(makeDefaultItem(() => mutateSetDefaultMfa(UserMfaMethod.Webauthn)));
      }
      items.push({
        text: m.profile_auth_card_disable_passkeys(),
        variant: 'danger',
        icon: 'delete',
        onClick: () =>
          openModal(ModalName.ConfirmAction, {
            title: m.modal_disable_mfa_passkeys_title(),
            contentMd: m.modal_disable_mfa_passkeys_content(),
            actionPromise: () =>
              Promise.all(
                securityKeys.map((key) =>
                  api.auth.mfa.webauthn.deleteKey({
                    username: user.username,
                    keyId: key.id,
                  }),
                ),
              ),
            invalidateKeys: invalidateAfterMfaChange.invalidate,
            submitProps: { text: m.controls_delete(), variant: 'critical' },
            onSuccess: () => Snackbar.default(m.modal_disable_mfa_passkeys_success()),
            onError: () => Snackbar.error(m.modal_disable_mfa_passkeys_error()),
          }),
      });
    }
    return toMenuGroup(items);
  }, [
    securityKeys,
    hasPasskeys,
    user.mfa_method,
    user.username,
    isSelf,
    mutateSetDefaultMfa,
    invalidateAfterMfaChange,
  ]);

  const totpMenuItems = useMemo(() => {
    const items: MenuItemProps[] = [];
    if (!user.totp_enabled && isSelf) {
      items.push({
        icon: 'check-circle',
        testId: 'enable-totp',
        text: m.controls_enable(),
        onClick: () => openModal('totpSetup'),
      });
    }
    if (user.totp_enabled) {
      if (isSelf && user.mfa_method !== UserMfaMethod.OneTimePassword) {
        items.push(
          makeDefaultItem(() => mutateSetDefaultMfa(UserMfaMethod.OneTimePassword)),
        );
      }
      items.push({
        icon: 'cancel',
        variant: 'danger',
        text: m.controls_disable(),
        onClick: () =>
          openModal(ModalName.ConfirmAction, {
            title: m.modal_disable_mfa_totp_title(),
            contentMd: m.modal_disable_mfa_totp_content(),
            actionPromise: () => api.user.mfa.totp.disable(user.username),
            invalidateKeys: invalidateAfterMfaChange.invalidate,
            submitProps: { text: m.controls_disable(), variant: 'critical' },
            onSuccess: () => Snackbar.default(m.modal_disable_mfa_totp_success()),
            onError: () => Snackbar.error(m.modal_disable_mfa_totp_error()),
          }),
      });
    }
    return toMenuGroup(items);
  }, [
    user.totp_enabled,
    user.mfa_method,
    user.username,
    isSelf,
    mutateSetDefaultMfa,
    invalidateAfterMfaChange,
  ]);

  return (
    <ProfileCard id="profile-auth-card">
      {!user.password_management_disabled && (
        <>
          <div className="section password">
            <p className="section-title">{m.profile_auth_card_section_password()}</p>
            <button
              type="button"
              className="text-action"
              data-testid="change-password"
              onClick={() =>
                openModal('changePassword', {
                  user,
                  adminForm: isCurrentUserAdmin && !isSelf,
                })
              }
            >
              {m.profile_auth_card_password_change()}
            </button>
          </div>
          <Divider orientation="horizontal" />
        </>
      )}
      <div className="section">
        <div className="header">
          <p className="section-title">{m.profile_auth_card_section_2fa()}</p>
          {isPresent(mfaMenuItems) && (
            <IconButtonMenu icon="menu" menuItems={mfaMenuItems} />
          )}
        </div>
        <div className="rows">
          <FactorRow
            icon="one-time-password"
            title={m.profile_auth_card_2fa_totp()}
            enabled={user.totp_enabled}
            isDefault={user.mfa_method === UserMfaMethod.OneTimePassword}
            menu={totpMenuItems}
            testId="totp-row"
          />
          <FactorRow
            icon="mail"
            title={m.profile_auth_card_2fa_email()}
            enabled={user.email_mfa_enabled}
            isDefault={user.mfa_method === UserMfaMethod.Email}
            menu={emailMenuItems}
            testId="email-codes-row"
            smtpDisabled={!smtpEnabled}
          />
        </div>
      </div>
      <Divider orientation="horizontal" />
      <div className="section">
        <div className="header" data-testid="passkeys-header">
          <div className="title">
            <p className="section-title">{m.profile_auth_card_section_passkeys()}</p>
            {hasPasskeys && user.mfa_method === UserMfaMethod.Webauthn && (
              <Badge
                variant="success"
                size="compact"
                text={m.profile_auth_card_default_method()}
              />
            )}
          </div>
          {isPresent(webauthnMenuItems) && (
            <IconButtonMenu icon="menu" menuItems={[webauthnMenuItems]} />
          )}
        </div>
        {hasPasskeys && (
          <div className="rows">
            {securityKeys.map((key) => (
              <WebauthnRow securityKey={key} username={user.username} key={key.id} />
            ))}
          </div>
        )}
      </div>
      <BiometricDevices />
    </ProfileCard>
  );
};
