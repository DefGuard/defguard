import { type ReactNode, useMemo } from 'react';
import './style.scss';
import clsx from 'clsx';
import { m } from '../../../../../paraglide/messages';
import type { OpenIdProviderKindValue } from '../../../../../shared/api/types';
import { externalProviderName } from '../../../../../shared/constants';
import { Button } from '../../../../../shared/defguard-ui/components/Button/Button';
import { Icon } from '../../../../../shared/defguard-ui/components/Icon';
import { ThemeVariable } from '../../../../../shared/defguard-ui/types';
import { isPresent } from '../../../../../shared/defguard-ui/utils/isPresent';
import google from './assets/google.png';
import jumpcloud from './assets/jumpcloud.png';
import microsoft from './assets/microsoft.png';
import okta from './assets/okta.png';
import zitadel from './assets/zitadel.png';

type Props = {
  provider: OpenIdProviderKindValue;
  displayName?: string;
  onClick: () => void;
  disabled?: boolean;
  edit?: boolean;
  loading?: boolean;
};

const providerImage: Record<OpenIdProviderKindValue, ReactNode> = {
  Custom: <Icon size={20} icon="lock-closed" staticColor={ThemeVariable.FgAction} />,
  Google: <img src={google} width={32} height={32} />,
  JumpCloud: <img src={jumpcloud} height={32} width={32} />,
  Microsoft: <img src={microsoft} height={28} width={28} />,
  Okta: <img src={okta} height={32} width={32} />,
  Zitadel: <img src={zitadel} height={28} width={28} />,
};

const providerDescription: Record<OpenIdProviderKindValue, string> = {
  Custom:
    'Enter the required details to link your account securely and manage logins with your custom setup.',
  Zitadel:
    'Get started with a multi-tenant, API-first identity platform with comprehensive SDKs that enable security, compliance, and extensibility.',
  JumpCloud:
    "Enable users to log in with their JumpCloud accounts through JumpCloud's secure directory and authentication platform.",
  Okta: "Allow users to sign in with their Okta accounts using Okta's secure identity management service.",
  Microsoft:
    "Enable users to log in with their Microsoft accounts through Microsoft's secure authentication platform.",
  Google:
    "Allow users to sign in securely with their Google accounts using Google's trusted authentication service.",
};

export const ExternalProviderCard = ({
  provider,
  displayName,
  onClick,
  edit = false,
  disabled = false,
  loading = false,
}: Props) => {
  const name = useMemo(() => {
    if (isPresent(displayName)) return displayName;
    return externalProviderName[provider];
  }, [displayName, provider]);

  return (
    <div className="external-provider-card">
      <div className="inner">
        <div className="icon-track">
          <div className={clsx('icon-box', `variant-${provider}`)}>
            {providerImage[provider]}
          </div>
        </div>
        <div className="content-track">
          <div className="top">
            <p className="name">{name}</p>
          </div>
          <p className="description">{providerDescription[provider]}</p>
        </div>
        <div className="action-track">
          {!edit && (
            <Button
              variant="primary"
              testId={`connect-${provider}`}
              text={m.controls_connect()}
              onClick={onClick}
              disabled={disabled}
            />
          )}
          {edit && (
            <Button
              variant="primary"
              iconLeft="edit"
              text={m.controls_edit()}
              onClick={onClick}
              disabled={disabled}
              loading={loading}
            />
          )}
        </div>
      </div>
    </div>
  );
};
