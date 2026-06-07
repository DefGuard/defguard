import './style.scss';
import clsx from 'clsx';
import type { ReactNode } from 'react';
import { m } from '../../../../../paraglide/messages';
import { Badge } from '../../../../../shared/defguard-ui/components/Badge/Badge';
import { Button } from '../../../../../shared/defguard-ui/components/Button/Button';
import { Icon } from '../../../../../shared/defguard-ui/components/Icon';
import { ThemeVariable } from '../../../../../shared/defguard-ui/types';
import google from '../../../SettingsExternalOpenIdPage/components/ExternalProviderCard/assets/google.png';
import microsoft from '../../../SettingsExternalOpenIdPage/components/ExternalProviderCard/assets/microsoft.png';

export type SmtpAuthCardVariant = 'basic' | 'custom' | 'google' | 'microsoft';

type Props = {
  variant: SmtpAuthCardVariant;
  active?: boolean;
  onApply: () => void;
};

const cardIcon: Record<SmtpAuthCardVariant, ReactNode> = {
  basic: <Icon size={20} icon="lock-closed" staticColor={ThemeVariable.FgAction} />,
  custom: <Icon size={20} icon="key" staticColor={ThemeVariable.FgAction} />,
  google: <img src={google} width={32} height={32} alt="Google" />,
  microsoft: <img src={microsoft} height={28} width={28} alt="Microsoft" />,
};

export const SmtpAuthMethodCard = ({ variant, active = false, onApply }: Props) => {
  const cardNames: Record<SmtpAuthCardVariant, string> = {
    basic: m.settings_smtp_auth_card_basic_name(),
    custom: m.settings_smtp_auth_card_custom_name(),
    google: m.settings_smtp_auth_card_google_name(),
    microsoft: m.settings_smtp_auth_card_microsoft_name(),
  };

  const cardDescriptions: Record<SmtpAuthCardVariant, string> = {
    basic: m.settings_smtp_auth_card_basic_description(),
    custom: m.settings_smtp_auth_card_custom_description(),
    google: m.settings_smtp_auth_card_google_description(),
    microsoft: m.settings_smtp_auth_card_microsoft_description(),
  };

  return (
    <div className={clsx('smtp-auth-method-card', { active })}>
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
                variant="success"
                text={m.settings_smtp_auth_card_applied_method()}
              />
            )}
          </div>
          <p className="description">{cardDescriptions[variant]}</p>
        </div>
        <div className="action-track">
          <Button variant="primary" text={m.controls_apply()} onClick={onApply} />
        </div>
      </div>
    </div>
  );
};
