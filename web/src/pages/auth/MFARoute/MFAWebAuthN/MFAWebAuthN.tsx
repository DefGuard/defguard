import {
  get,
  parseRequestOptionsFromJSON,
} from '@github/webauthn-json/browser-ponyfill';
import { useMutation } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import { useNavigate } from 'react-router';
import shallow from 'zustand/shallow';

import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../shared/components/layout/Button/Button';
import { useAuthStore } from '../../../../shared/hooks/store/useAuthStore';
import useApi from '../../../../shared/hooks/useApi';
import { MutationKeys } from '../../../../shared/mutations';
import { toaster } from '../../../../shared/utils/toaster';
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

  const logIn = useAuthStore((state) => state.logIn);
  const clearMFAStore = useMFAStore((state) => state.resetState);
  const navigate = useNavigate();
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
        clearMFAStore();
        logIn(data);
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
            toaster.error('Failed to read key. Try again.');
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
      <p>When you are ready to authenticate, press the button below.</p>
      <Button
        text="Use security key"
        loading={mfaStartLoading || mfaFinishLoading || awaitingKey}
        onClick={() => mfaStart()}
        size={ButtonSize.BIG}
        styleVariant={ButtonStyleVariant.PRIMARY}
      />
      {totpAvailable || web3Available ? (
        <nav>
          <span>or</span>
          {totpAvailable && (
            <Button
              text="Use authenticator app instead"
              size={ButtonSize.BIG}
              onClick={() => navigate('../totp')}
            />
          )}
          {web3Available && (
            <Button
              text="Use your wallet instead"
              size={ButtonSize.BIG}
              onClick={() => navigate('../web3')}
            />
          )}
        </nav>
      ) : null}
    </>
  );
};
