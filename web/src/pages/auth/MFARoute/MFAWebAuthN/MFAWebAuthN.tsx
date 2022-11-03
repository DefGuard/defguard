import {
  get,
  parseRequestOptionsFromJSON,
} from '@github/webauthn-json/browser-ponyfill';
import { useMutation } from '@tanstack/react-query';
import { useState } from 'react';
import { useNavigate } from 'react-router';

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
          .catch(() => toaster.error('Failed to read key. Try again.'))
          .finally(() => setAwaitingKey(false));
      },
    }
  );

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
      <nav>
        <span>or</span>
        <Button
          text="Use authenticator app instead"
          size={ButtonSize.BIG}
          onClick={() => navigate('../totp')}
        />
        <Button
          text="Use your wallet instead"
          size={ButtonSize.BIG}
          onClick={() => navigate('../web3')}
        />
      </nav>
    </>
  );
};
