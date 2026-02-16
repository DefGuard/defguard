import { Fragment, useMemo } from 'react';
import { m } from '../../../../../../../paraglide/messages';
import { Button } from '../../../../../../../shared/defguard-ui/components/Button/Button';
import { Divider } from '../../../../../../../shared/defguard-ui/components/Divider/Divider';
import { Icon } from '../../../../../../../shared/defguard-ui/components/Icon';
import type { IconKindValue } from '../../../../../../../shared/defguard-ui/components/Icon/icon-types';
import { IconButtonMenu } from '../../../../../../../shared/defguard-ui/components/IconButtonMenu/IconButtonMenu';
import type {
  MenuItemProps,
  MenuItemsGroup,
} from '../../../../../../../shared/defguard-ui/components/Menu/types';
import { isPresent } from '../../../../../../../shared/defguard-ui/utils/isPresent';
import { ProfileCard } from '../../../../components/ProfileCard/ProfileCard';
import './style.scss';
import { type QueryKey, useMutation } from '@tanstack/react-query';
import { Link } from '@tanstack/react-router';
import { cloneDeep } from 'lodash-es';
import api from '../../../../../../../shared/api/api';
import {
  type SecurityKey,
  UserMfaMethod,
  type UserMfaMethodValue,
} from '../../../../../../../shared/api/types';
import { Badge } from '../../../../../../../shared/defguard-ui/components/Badge/Badge';
import { TooltipContent } from '../../../../../../../shared/defguard-ui/providers/tooltip/TooltipContent';
import { TooltipProvider } from '../../../../../../../shared/defguard-ui/providers/tooltip/TooltipContext';
import { TooltipTrigger } from '../../../../../../../shared/defguard-ui/providers/tooltip/TooltipTrigger';
import { ThemeSpacing } from '../../../../../../../shared/defguard-ui/types';
import { openModal } from '../../../../../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../../../../../shared/hooks/modalControls/modalTypes';
import { useApp } from '../../../../../../../shared/hooks/useApp';
import { useAuth } from '../../../../../../../shared/hooks/useAuth';
import { useUserProfile } from '../../../../hooks/useUserProfilePage';
import { UserProfileTab } from '../../../types';

export const ProfileAuthCard = () => {
  const securityKeys = useUserProfile((s) => s.security_keys);
  const user = useUserProfile((s) => s.user);
  const authUsername = useAuth((s) => s.user?.username as string);
  const smtpEnabled = useApp((s) => s.appInfo.smtp_enabled);
  const devices = useUserProfile((s) => s.devices);
  const biometricDevices = useMemo(
    () => devices.filter((device) => device.biometry_enabled),
    [devices],
  );

  const invalidateAfterMfaChange = useMemo(() => {
    const res: QueryKey[] = [];
    if (user.username === authUsername) {
      res.push(['me']);
    }
    res.push(['user', user.username]);
    return {
      invalidate: res,
    };
  }, [authUsername, user.username]);

  const { mutate: mutateSetDefaultMfa } = useMutation({
    mutationFn: (method: UserMfaMethodValue) => {
      const data = cloneDeep(user);
      data.mfa_method = method;
      return api.user.editUser({
        username: data.username,
        body: data,
      });
    },
    meta: invalidateAfterMfaChange,
  });

  const { mutate: disableMfaMutation } = useMutation({
    mutationFn: () => {
      if (user.username === authUsername) {
        return api.auth.mfa.disable();
      }
      return api.user.disableMfa(user.username);
    },
    meta: invalidateAfterMfaChange,
  });

  const { mutate: mutateEnableMfa } = useMutation({
    mutationFn: api.auth.mfa.enable,
    meta: invalidateAfterMfaChange,
  });

  const { mutate: mutateDisableEmailMfa } = useMutation({
    mutationFn: api.user.mfa.email.disable,
    meta: invalidateAfterMfaChange,
  });

  const { mutate: mutateDisableTotp } = useMutation({
    mutationFn: api.user.mfa.totp.disable,
    meta: invalidateAfterMfaChange,
  });

  const { mutate: mutateDisableWebauthn } = useMutation({
    mutationFn: () => {
      const res = securityKeys.map((key) =>
        api.auth.mfa.webauthn.deleteKey({
          username: user.username,
          keyId: key.id,
        }),
      );
      return Promise.all(res);
    },
    meta: invalidateAfterMfaChange,
  });
  const emailMenuItems = useMemo(() => {
    const items: MenuItemProps[] = [];
    if (!user.email_mfa_enabled && user.username === authUsername) {
      items.push({
        testId: 'enable-email',
        text: m.controls_enable(),
        icon: 'check-circle',
        onClick: () => openModal('emailMfaSetup'),
      });
    }
    if (user.email_mfa_enabled) {
      if (user.mfa_method !== UserMfaMethod.Email) {
        items.push({
          icon: 'check',
          text: m.profile_auth_card_make_default(),
          onClick: () => mutateSetDefaultMfa(UserMfaMethod.Email),
        });
      }
      items.push({
        text: m.controls_disable(),
        icon: 'minus-circle',
        onClick: () => mutateDisableEmailMfa(user.username),
      });
    }
    const res: MenuItemsGroup = {
      items,
    };
    return items.length > 0 ? res : null;
  }, [
    user.email_mfa_enabled,
    mutateDisableEmailMfa,
    mutateSetDefaultMfa,
    user.mfa_method,
    user.username,
    authUsername,
  ]);

  const mfaMenuItems = useMemo(() => {
    const hasConfiguredMfa =
      user.email_mfa_enabled || user.totp_enabled || securityKeys.length > 0;
    if (!hasConfiguredMfa) return null;
    const res: MenuItemsGroup[] = [];
    /** user configured mfa but recovery keys modal was not confirmed
     *  user needs to accept new recovery keys
     */
    if (hasConfiguredMfa && !user.mfa_enabled) {
      res.push({
        items: [
          {
            text: m.controls_enable(),
            icon: 'check-circle',
            onClick: mutateEnableMfa,
          },
        ],
      });
      return res;
    }
    // mfa is enabled and at least one SSO is configured
    res.push({
      items: [
        {
          icon: 'disabled',
          variant: 'danger',
          text: m.profile_auth_card_2fa_controls_disable_all(),
          onClick: disableMfaMutation,
        },
      ],
    });
    return res;
  }, [
    securityKeys.length,
    user.email_mfa_enabled,
    user.mfa_enabled,
    user.totp_enabled,
    mutateEnableMfa,
    disableMfaMutation,
  ]);

  const webauthnMenuItems = useMemo(() => {
    const items: MenuItemProps[] = [];
    if (user.username === authUsername) {
      items.push({
        text: m.profile_auth_card_add_passkey(),
        icon: 'plus-circle',
        testId: 'add-passkey',
        onClick: () => openModal(ModalName.WebauthnSetup),
      });
    }
    if (securityKeys.length) {
      if (user.mfa_method !== UserMfaMethod.Webauthn) {
        items.push({
          icon: 'check',
          text: m.profile_auth_card_make_default(),
          onClick: () => mutateSetDefaultMfa(UserMfaMethod.Webauthn),
        });
      }
      items.push({
        text: m.profile_auth_card_disable_passkeys(),
        variant: 'danger',
        icon: 'delete',
        onClick: () => mutateDisableWebauthn(),
      });
    }
    return items.length > 0 ? { items } : null;
  }, [
    mutateDisableWebauthn,
    securityKeys.length,
    mutateSetDefaultMfa,
    user.mfa_method,
    user.username,
    authUsername,
  ]);

  const totpMenuItems = useMemo(() => {
    const items: MenuItemProps[] = [];
    if (!user.totp_enabled && user.username === authUsername) {
      items.push({
        icon: 'check-circle',
        testId: 'enable-totp',
        text: m.controls_enable(),
        onClick: () => {
          openModal('totpSetup');
        },
      });
    }
    if (user.totp_enabled) {
      if (user.mfa_method !== UserMfaMethod.OneTimePassword) {
        items.push({
          icon: 'check',
          text: m.profile_auth_card_make_default(),
          onClick: () => mutateSetDefaultMfa(UserMfaMethod.OneTimePassword),
        });
      }
      items.push({
        icon: 'minus-circle',
        text: m.controls_disable(),
        onClick: () => mutateDisableTotp(user.username),
      });
    }

    return items.length > 0 ? { items } : null;
  }, [
    mutateDisableTotp,
    user.totp_enabled,
    mutateSetDefaultMfa,
    user.mfa_method,
    user.username,
    authUsername,
  ]);
  return (
    <ProfileCard id="profile-auth-card">
      <h2>{m.profile_auth_card_title()}</h2>
      <div className="section">
        <div className="header">
          <p className="section-title">{m.profile_auth_card_section_password()}</p>
        </div>
        <Button
          variant="outlined"
          iconLeft="lock-open"
          text={m.profile_auth_card_password_change()}
          testId="change-password"
          onClick={() => {
            // open admin form only if admin and is not editing self
            openModal('changePassword', {
              user,
              adminForm: user.is_admin && user.username !== authUsername,
            });
          }}
        />
      </div>
      <Divider orientation="horizontal" />
      <div className="section">
        <div className="header">
          <p className="section-title">{m.profile_auth_card_section_2fa()}</p>
          {isPresent(mfaMenuItems) && (
            <IconButtonMenu icon="menu" menuItems={mfaMenuItems} />
          )}
        </div>
        <FactorRow
          isDefault={user.mfa_method === 'OneTimePassword'}
          icon="one-time-password"
          availability="both"
          title={m.profile_auth_card_2fa_totp()}
          enabled={user.totp_enabled}
          menu={totpMenuItems}
          testId="totp-row"
        />
        <Divider />
        <FactorRow
          icon="mail"
          availability="both"
          title={m.profile_auth_card_2fa_email()}
          enabled={user.email_mfa_enabled}
          isDefault={user.mfa_method === 'Email'}
          menu={emailMenuItems}
          testId="email-codes-row"
          smtpDisabled={!smtpEnabled}
        />
        <Divider />
        <FactorRow
          icon="access-settings"
          availability="sso"
          title={m.profile_auth_card_2fa_passkeys()}
          enabled={securityKeys.length > 0}
          isDefault={user.mfa_method === 'Webauthn'}
          menu={webauthnMenuItems}
          testId="passkeys-row"
        />
        {securityKeys.length > 0 && (
          <div className="webauthn-keys">
            {securityKeys.map((key) => (
              <WebauthnRow securityKey={key} username={user.username} key={key.id} />
            ))}
          </div>
        )}
        {biometricDevices.length > 0 && (
          <>
            <Divider spacing={ThemeSpacing.Xl} />
            <div className="biometric-devices section">
              <div className="top">
                <p className="section-title">{m.profile_auth_card_biometric_title()}</p>
                <Link
                  from="/user/$username"
                  to="/user/$username"
                  search={{
                    tab: UserProfileTab.Devices,
                  }}
                >
                  {m.profile_auth_card_devices_link()}
                </Link>
              </div>
              {biometricDevices.map((device, index) => (
                <Fragment key={device.id}>
                  <div className="device">
                    <Icon icon="biometric" />
                    <p>{device.name}</p>
                  </div>
                  {index !== biometricDevices.length - 1 && (
                    <Divider spacing={ThemeSpacing.Lg} />
                  )}
                </Fragment>
              ))}
            </div>
          </>
        )}
      </div>
    </ProfileCard>
  );
};

const WebauthnRow = ({
  securityKey,
  username,
}: {
  securityKey: SecurityKey;
  username: string;
}) => {
  const { mutate } = useMutation({
    mutationFn: api.auth.mfa.webauthn.deleteKey,
    meta: {
      invalidate: [['user', username]],
    },
  });

  const menuItems = useMemo(() => {
    const items: MenuItemProps[] = [];
    items.push({
      text: m.profile_auth_card_delete_passkey(),
      icon: 'delete',
      variant: 'danger',
      onClick: () =>
        mutate({
          keyId: securityKey.id,
          username,
        }),
    });
    return {
      items,
    };
  }, [mutate, securityKey.id, username]);

  return (
    <div className="webauthn-row">
      <p className="name">{securityKey.name}</p>
      <div className="controls">
        <IconButtonMenu icon="menu" menuItems={[menuItems]} />
      </div>
    </div>
  );
};

interface FactorRowProps {
  icon: IconKindValue;
  title: string;
  enabled: boolean;
  isDefault: boolean;
  availability: 'sso' | 'both' | 'mfa';
  menu?: MenuItemsGroup | null;
  testId?: string;
  smtpDisabled?: boolean;
}

const FactorRow = ({
  enabled,
  isDefault,
  icon,
  title,
  menu,
  testId,
  availability,
  smtpDisabled,
}: FactorRowProps) => {
  const menuItems = useMemo(() => (menu ? [menu] : undefined), [menu]);

  const availabilityText = useMemo(() => {
    switch (availability) {
      case 'both':
        return 'SSO/MFA';
      case 'mfa':
        return 'MFA';
      case 'sso':
        return 'SSO';
    }
  }, [availability]);
  const showSmtpDisabledWarning = useMemo(
    () => isPresent(smtpDisabled) && smtpDisabled,
    [smtpDisabled],
  );

  return (
    <div className="factor-row" data-testid={testId}>
      <div className="content-track">
        <div className="row info">
          <Icon icon={icon} />
          <p className="factor-name">{title}</p>
          <div className="badges">
            {showSmtpDisabledWarning && (
              <Badge variant="critical" text={m.state_not_configured()} />
            )}
            {!enabled && !showSmtpDisabledWarning && (
              <Badge variant="warning" text={m.state_disabled()} />
            )}
            {enabled && <Badge variant="success" text={m.state_enabled()} />}
            {isDefault && <Badge variant="neutral" text={m.state_default()} />}
          </div>
        </div>
        <div className="row availability">
          <div className="fill"></div>
          <TooltipProvider>
            <TooltipTrigger>
              <p className="availability">
                {showSmtpDisabledWarning
                  ? m.state_smtp_not_configured()
                  : availabilityText}
              </p>
            </TooltipTrigger>
            <TooltipContent>
              <p>{m.test_placeholder()}</p>
            </TooltipContent>
          </TooltipProvider>
        </div>
      </div>
      <div className="controls">
        {isPresent(menuItems) && !showSmtpDisabledWarning && (
          <IconButtonMenu icon="menu" menuItems={menuItems} />
        )}
      </div>
    </div>
  );
};
