import { useMemo } from 'react';
import { useMatch, useNavigate } from 'react-router';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { Button } from '../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../shared/defguard-ui/components/Layout/Button/types';
import { UserMFAMethod } from '../../../../shared/types';
import { useMFAStore } from '../../shared/hooks/useMFAStore';

export const MFANav = () => {
  const { LL } = useI18nContext();
  const localLL = LL.loginPage.mfa;

  const totpRoute = useMatch('/auth/mfa/totp');
  const webAuthNRoute = useMatch('/auth/mfa/webauthn');
  const emailRoute = useMatch('/auth/mfa/email');
  const recoveryRoute = useMatch('/auth/mfa/recovery');

  const navigate = useNavigate();

  const [emailAvailable, totpAvailable, webauthnAvailable] = useMFAStore(
    (state) => [state.email_available, state.totp_available, state.webauthn_available],
    shallow,
  );

  const availableMethods = useMemo((): UserMFAMethod[] => {
    const res: UserMFAMethod[] = [];
    if (emailAvailable && !emailRoute) {
      res.push(UserMFAMethod.EMAIL);
    }
    if (totpAvailable && !totpRoute) {
      res.push(UserMFAMethod.ONE_TIME_PASSWORD);
    }
    if (webauthnAvailable && !webAuthNRoute) {
      res.push(UserMFAMethod.WEB_AUTH_N);
    }
    return res;
  }, [
    totpRoute,
    emailRoute,
    totpAvailable,
    emailAvailable,
    webauthnAvailable,
    webAuthNRoute,
  ]);

  const getLinks = useMemo((): MFALink[] => {
    let res: MFALink[] = [
      {
        key: 0,
        text: localLL.controls.useEmail(),
        link: '/auth/mfa/email',
        type: UserMFAMethod.EMAIL,
      },
      {
        key: 1,
        text: localLL.controls.useAuthenticator(),
        link: '/auth/mfa/totp',
        type: UserMFAMethod.ONE_TIME_PASSWORD,
      },
      {
        key: 2,
        text: localLL.controls.useWebauthn(),
        link: '/auth/mfa/webauthn',
        type: UserMFAMethod.WEB_AUTH_N,
      },
    ];

    res = res.filter((link) => availableMethods.includes(link.type));

    return res;
  }, [localLL.controls, availableMethods]);

  return (
    <nav>
      <span>{LL.common.conditions.or()}</span>
      {getLinks.map((link) => (
        <Button
          key={link.key}
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.LINK}
          text={link.text}
          onClick={() => navigate(link.link, { replace: true })}
        />
      ))}
      {!recoveryRoute && (
        <Button
          text={LL.loginPage.mfa.controls.useRecoveryCode()}
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.LINK}
          onClick={() => navigate('/auth/mfa/recovery', { replace: true })}
        />
      )}
    </nav>
  );
};

type MFALink = {
  text: string;
  link: string;
  type: UserMFAMethod;
  key: string | number;
};
