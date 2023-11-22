import { get, parseRequestOptionsFromJSON } from '@github/webauthn-json/browser-ponyfill';
import { useMutation } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import { useNavigate } from 'react-router';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { Button } from '../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../shared/defguard-ui/components/Layout/Button/types';
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
  const webauthnAvailable = useMFAStore((state) => state.webauthn_available);

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
    </>
  );
};
