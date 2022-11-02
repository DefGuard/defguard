import {
  get,
  parseRequestOptionsFromJSON,
} from '@github/webauthn-json/browser-ponyfill';
import { useMutation } from '@tanstack/react-query';
import { useState } from 'react';

import Button from '../../../../shared/components/layout/Button/Button';
import useApi from '../../../../shared/hooks/useApi';
import { MutationKeys } from '../../../../shared/mutations';
import { toaster } from '../../../../shared/utils/toaster';

export const MFAKey = () => {
  const [awaitingKey, setAwaitingKey] = useState(false);
  const {
    auth: {
      mfa: {
        webauthn: { start, finish },
      },
    },
  } = useApi();

  const { mutate: mfaFinish, isLoading: mfaFinishLoading } = useMutation(
    [MutationKeys.WEBAUTHN_MFA_FINISH],
    finish,
    {
      onSuccess: () => { },
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
      />
      <div className="mfa-choices">
        <Button text="Use authenticator app instead" />
        <Button text="Use your wallet instead" />
      </div>
    </>
  );
};
