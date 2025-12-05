import { useMutation } from '@tanstack/react-query';
import { useCallback } from 'react';
import { m } from '../../../paraglide/messages';
import api from '../../../shared/api/api';
import { LoginPage } from '../../../shared/components/LoginPage/LoginPage';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import { useAuth } from '../../../shared/hooks/useAuth';
import { MfaLinks } from '../shared/MfaLinks/MfaLinks';

export const LoginWebauthn = () => {
  const loginPromise = useCallback(async () => {
    const backendStartResponse = await api.auth.mfa.webauthn.login.start();
    const pkc = PublicKeyCredential.parseRequestOptionsFromJSON(
      backendStartResponse.data.publicKey,
    );
    const navigatorData = await navigator.credentials.get({
      publicKey: pkc,
    });
    if (navigatorData != null) {
      const requestData = (navigatorData as PublicKeyCredential).toJSON();
      const response = await api.auth.mfa.webauthn.login.finish(requestData);
      useAuth.getState().setUser(response.data.user);
    }
  }, []);

  const { mutate, isPending } = useMutation({
    mutationFn: loginPromise,
  });

  return (
    <LoginPage id="webauthn-login-page">
      <h1>{m.login_mfa_title()}</h1>
      <h2>{m.login_mfa_passkey_subtitle()}</h2>
      <SizedBox height={ThemeSpacing.Xl5} />
      <Button
        size="big"
        variant="primary"
        text={m.login_mfa_passkey_button()}
        onClick={() => mutate()}
        testId="login-with-passkey"
        loading={isPending}
      />
      <MfaLinks />
    </LoginPage>
  );
};
