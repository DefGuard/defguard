import './style.scss';
import clsx from 'clsx';
import type { ReactNode } from 'react';
import { m } from '../../../../../paraglide/messages';
import { BusinessBadge } from '../../../../../shared/components/badges/BusinessBadge';
import { Badge } from '../../../../../shared/defguard-ui/components/Badge/Badge';
import { Button } from '../../../../../shared/defguard-ui/components/Button/Button';
import { Icon, IconKind } from '../../../../../shared/defguard-ui/components/Icon';
import { IconButtonMenu } from '../../../../../shared/defguard-ui/components/IconButtonMenu/IconButtonMenu';
import type { MenuItemsGroup } from '../../../../../shared/defguard-ui/components/Menu/types';
import { ThemeVariable } from '../../../../../shared/defguard-ui/types';
import google from '../../../SettingsExternalOpenIdPage/components/ExternalProviderCard/assets/google.png';
import microsoft from '../../../SettingsExternalOpenIdPage/components/ExternalProviderCard/assets/microsoft.png';

export type SmtpAuthCardVariant = 'none' | 'basic' | 'google' | 'microsoft' | 'custom';

type Props = {
  variant: SmtpAuthCardVariant;
  active?: boolean;
  locked?: boolean;
  onConfigure: () => void;
  onEdit: () => void;
  onSendTestEmail: () => void;
  onDelete: () => void;
};

const cardIcon: Record<SmtpAuthCardVariant, ReactNode> = {
  none: <Icon size={20} icon="lock-open" staticColor={ThemeVariable.FgAction} />,
  basic: <Icon size={20} icon="key" staticColor={ThemeVariable.FgAction} />,
  custom: <Icon size={20} icon="lock-closed" staticColor={ThemeVariable.FgAction} />,
  google: <img src={google} width={32} height={32} alt="Google" />,
  microsoft: <img src={microsoft} height={28} width={28} alt="Microsoft" />,
};

export const SmtpAuthMethodCard = ({
  variant,
  active = false,
  locked = false,
  onConfigure,
  onEdit,
  onSendTestEmail,
  onDelete,
}: Props) => {
  const cardNames: Record<SmtpAuthCardVariant, string> = {
    none: m.settings_smtp_auth_card_none_name(),
    basic: m.settings_smtp_auth_card_basic_name(),
    custom: m.settings_smtp_auth_card_custom_name(),
    google: m.settings_smtp_auth_card_google_name(),
    microsoft: m.settings_smtp_auth_card_microsoft_name(),
  };

  const cardDescriptions: Record<SmtpAuthCardVariant, string> = {
    none: m.settings_smtp_auth_card_none_description(),
    basic: m.settings_smtp_auth_card_basic_description(),
    custom: m.settings_smtp_auth_card_custom_description(),
    google: m.settings_smtp_auth_card_google_description(),
    microsoft: m.settings_smtp_auth_card_microsoft_description(),
  };

  const menuItems: MenuItemsGroup[] = [
    {
      items: [
        {
          text: m.controls_edit(),
          icon: 'edit',
          testId: 'smtp-card-edit',
          onClick: onEdit,
        },
        {
          text: m.settings_smtp_button_send_test_email(),
          icon: 'mail',
          testId: 'smtp-card-send-test-email',
          onClick: onSendTestEmail,
        },
      ],
    },
    {
      items: [
        {
          text: m.controls_delete(),
          icon: 'delete',
          testId: 'smtp-card-delete',
          variant: 'danger',
          onClick: onDelete,
        },
      ],
    },
  ];

  return (
    <div className={clsx('smtp-auth-method-card')}>
      <div className="inner">
        <div className="icon-track">
          <div className={clsx('icon-box', `variant-${variant}`)}>
            {cardIcon[variant]}
          </div>
        </div>
        <div className="content-track">
          <div className="top">
            <p className="name">{cardNames[variant]}</p>
            {active && (
              <Badge
                showIcon
                icon={IconKind.CheckFilled}
                variant="success"
                text={m.settings_smtp_auth_card_active_method()}
              />
            )}
            {locked && <BusinessBadge />}
          </div>
          <p className="description">{cardDescriptions[variant]}</p>
        </div>
        <div className="action-track">
          {active ? (
            <IconButtonMenu icon="menu" menuItems={menuItems} />
          ) : (
            <Button
              variant="outlined"
              text={m.controls_configure()}
              onClick={onConfigure}
            />
          )}
        </div>
      </div>
    </div>
  );
};
