import { useMemo } from 'react';
import { m } from '../../../../../../../paraglide/messages';
import { Button } from '../../../../../../../shared/defguard-ui/components/Button/Button';
import { Divider } from '../../../../../../../shared/defguard-ui/components/Divider/Divider';
import { Icon } from '../../../../../../../shared/defguard-ui/components/Icon';
import type { IconKindValue } from '../../../../../../../shared/defguard-ui/components/Icon/icon-types';
import { IconButtonMenu } from '../../../../../../../shared/defguard-ui/components/IconButtonMenu/IconButtonMenu';
import type { MenuItemsGroup } from '../../../../../../../shared/defguard-ui/components/Menu/types';
import { isPresent } from '../../../../../../../shared/defguard-ui/utils/isPresent';
import { ProfileCard } from '../../../../components/ProfileCard/ProfileCard';
import './style.scss';
import { Badge } from '../../../../../../../shared/defguard-ui/components/Badge/Badge';
import { openModal } from '../../../../../../../shared/hooks/modalControls/modalsSubjects';
import { useAuth } from '../../../../../../../shared/hooks/useAuth';
import { useUserProfile } from '../../../../hooks/useUserProfilePage';

export const ProfileAuthCard = () => {
  const securityKeys = useUserProfile((s) => s.profile.security_keys);
  const user = useUserProfile((s) => s.profile.user);
  const authUsername = useAuth((s) => s.user?.username as string);

  const totpMenuItems = useMemo(() => {
    const res: MenuItemsGroup = {
      items: [
        {
          icon: 'check-circle',
          text: m.controls_enable(),
          onClick: () => {
            openModal('totpSetup');
          },
        },
      ],
    };
    return res;
  }, []);

  return (
    <ProfileCard id="profile-auth-card">
      <h2>{m.profile_auth_card_title()}</h2>
      <div className="section">
        <p>{m.profile_auth_card_section_password()}</p>
        <Button
          variant="outlined"
          iconLeft="lock-open"
          text={m.profile_auth_card_password_change()}
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
        <p>{m.profile_auth_card_section_2fa()}</p>
        <FactorRow
          icon="one-time-password"
          title={m.profile_auth_card_2fa_totp()}
          enabled={user.totp_enabled}
          menu={totpMenuItems}
        />
        <Divider />
        <FactorRow
          icon="mail"
          title={m.profile_auth_card_2fa_email()}
          enabled={user.email_mfa_enabled}
        />
        <Divider />
        <FactorRow
          icon="access-settings"
          title={m.profile_auth_card_2fa_passkeys()}
          enabled={securityKeys.length > 0}
        />
      </div>
    </ProfileCard>
  );
};

interface FactorRowProps {
  icon: IconKindValue;
  title: string;
  enabled: boolean;
  menu?: MenuItemsGroup;
  testId?: string;
}

const FactorRow = ({ enabled, icon, title, menu, testId }: FactorRowProps) => {
  const menuItems = useMemo(() => (menu ? [menu] : undefined), [menu]);

  return (
    <div className="factor-row" data-testid={testId}>
      <Icon icon={icon} />
      <p>{title}</p>
      <div className="badges">
        {!enabled && <Badge variant="warning" text={m.state_disabled()} />}
        {enabled && <Badge variant="success" text={m.state_enabled()} />}
      </div>
      <div className="controls">
        {isPresent(menuItems) && <IconButtonMenu icon="menu" menuItems={menuItems} />}
      </div>
    </div>
  );
};
