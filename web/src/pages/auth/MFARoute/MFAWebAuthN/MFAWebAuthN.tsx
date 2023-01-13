import {
  get,
  parseRequestOptionsFromJSON,
} from '@github/webauthn-json/browser-ponyfill';
import { useMutation } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import { useNavigate } from 'react-router';
import shallow from 'zustand/shallow';
import { useI18nContext } from '../../../../i18n/i18n-react';

import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../shared/components/layout/Button/Button';
import { useAuthStore } from '../../../../shared/hooks/store/useAuthStore';
import { useOpenIDStore } from '../../../../shared/hooks/store/useOpenIdStore';
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

  const logIn = useAuthStore((state) => state.logIn);
  const setOpenIDStore = useOpenIDStore((state) => state.setOpenIDStore)
  const clearMFAStore = useMFAStore((state) => state.resetState);
  const navigate = useNavigate();
  const toaster = useToaster();
  const [webauthnAvailable, web3Available, totpAvailable] = useMFAStore(
    (state) => [
      state.webauthn_available,
      state.web3_available,
      state.totp_available,
    ],
    shallow
  );

  const { mutate: mfaFinish, isLoading: mfaFinishLoading } = useMutation(
    [MutationKeys.WEBAUTHN_MFA_FINISH],
    finish,
    {
      onSuccess: (data) => {
        const { user, url } = data;
        if (user && url) {
          clearMFAStore();
					setOpenIDStore({openIDRedirect: true})
          window.location.replace(url);
          return;
        }
        if (user) {
          clearMFAStore();
          logIn(user);
        }
      },
    }
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
    }
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
        size={ButtonSize.BIG}
        styleVariant={ButtonStyleVariant.PRIMARY}
      />
      <nav>
        <span>or</span>
        {totpAvailable && (
          <Button
            text={LL.loginPage.mfa.controls.useAuthenticator()}
            size={ButtonSize.BIG}
            onClick={() => navigate('../totp')}
          />
        )}
        {web3Available && (
          <Button
            text={LL.loginPage.mfa.controls.useWallet()}
            size={ButtonSize.BIG}
            onClick={() => navigate('../web3')}
          />
        )}
        <Button
          text={LL.loginPage.mfa.controls.useRecoveryCode()}
          size={ButtonSize.BIG}
          styleVariant={ButtonStyleVariant.LINK}
          onClick={() => navigate('../recovery')}
        />
      </nav>
    </>
  );
};
