import { get, parseRequestOptionsFromJSON } from '@github/webauthn-json/browser-ponyfill';
import { useMutation } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import { useNavigate } from 'react-router';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { Button } from '../../../../shared/components/layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../shared/components/layout/Button/types';
import { useAuthStore } from '../../../../shared/hooks/store/useAuthStore';
import useApi from '../../../../shared/hooks/useApi';
import { useToaster } from '../../../../shared/hooks/useToaster';
import { MutationKeys } from '../../../../shared/mutations';
import { useMFAStore } from '../../shared/hooks/useMFAStore';

export const MFAWebAuthN = () => {
  const [awaitingKey, setAwaitingKey] = useState(false);
  const {
    auth: {
      mfa: {
        webauthn: { start, finish },
      },
    },
  } = useApi();
  const { LL } = useI18nContext();

  const loginSubject = useAuthStore((state) => state.loginSubject);

  const navigate = useNavigate();
  const toaster = useToaster();
  const [webauthnAvailable, web3Available, totpAvailable] = useMFAStore(
    (state) => [state.webauthn_available, state.web3_available, state.totp_available],
    shallow,
  );

  const { mutate: mfaFinish, isLoading: mfaFinishLoading } = useMutation(
    [MutationKeys.WEBAUTHN_MFA_FINISH],
    finish,
    {
      onSuccess: (data) => loginSubject.next(data),
      onError: (err) => {
        toaster.error(LL.messages.error());
        console.error(err);
      },
    },
  );

  const { mutate: mfaStart, isLoading: mfaStartLoading } = useMutation(
    [MutationKeys.WEBAUTHN_MFA_START],
    start,
    {
      onSuccess: async (data) => {
        setAwaitingKey(true);
        const parsed = parseRequestOptionsFromJSON(data);
        get(parsed)
          .then((response) => mfaFinish(response.toJSON()))
          .catch((err) => {
            toaster.error(LL.loginPage.mfa.webauthn.messages.error());
            console.error(err);
          })
          .finally(() => setAwaitingKey(false));
      },
    },
  );

  useEffect(() => {
    if (!webauthnAvailable) {
      navigate('../');
    }
  }, [navigate, webauthnAvailable]);

  return (
    <>
      <p>{LL.loginPage.mfa.webauthn.header()}</p>
      <Button
        text={LL.loginPage.mfa.webauthn.controls.submit()}
        loading={mfaStartLoading || mfaFinishLoading || awaitingKey}
        onClick={() => mfaStart()}
        size={ButtonSize.LARGE}
        styleVariant={ButtonStyleVariant.PRIMARY}
      />
      <nav>
        <span>or</span>
        {totpAvailable && (
          <Button
            text={LL.loginPage.mfa.controls.useAuthenticator()}
            size={ButtonSize.LARGE}
            onClick={() => navigate('../totp')}
          />
        )}
        {web3Available && (
          <Button
            text={LL.loginPage.mfa.controls.useWallet()}
            size={ButtonSize.LARGE}
            onClick={() => navigate('../web3')}
          />
        )}
        <Button
          text={LL.loginPage.mfa.controls.useRecoveryCode()}
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.LINK}
          onClick={() => navigate('../recovery')}
        />
      </nav>
    </>
  );
};
