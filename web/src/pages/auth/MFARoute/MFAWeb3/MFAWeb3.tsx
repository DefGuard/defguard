import { useMutation } from '@tanstack/react-query';
import { useEffect } from 'react';
import { useNavigate } from 'react-router';
import { useAccount, useSignMessage, useSignTypedData } from 'wagmi';
import shallow from 'zustand/shallow';
import { useI18nContext } from '../../../../i18n/i18n-react';

import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../shared/components/layout/Button/Button';
import { useAuthStore } from '../../../../shared/hooks/store/useAuthStore';
import { useModalStore } from '../../../../shared/hooks/store/useModalStore';
import { useOpenIDStore } from '../../../../shared/hooks/store/useOpenIdStore';
import useApi from '../../../../shared/hooks/useApi';
import { useToaster } from '../../../../shared/hooks/useToaster';
import { MutationKeys } from '../../../../shared/mutations';
import { useMFAStore } from '../../shared/hooks/useMFAStore';

export const MFAWeb3 = () => {
  const {
    auth: {
      mfa: {
        web3: { start, finish },
      },
    },
  } = useApi();
  const { LL } = useI18nContext();

  const { isConnected, isConnecting, address } = useAccount();
  const setModalsState = useModalStore((state) => state.setState);
  const logIn = useAuthStore((state) => state.logIn);
  const setOpenIDStore = useOpenIDStore((state) => state.setOpenIDStore);
  const resetMFAStore = useMFAStore((state) => state.resetState);
  const toaster = useToaster();
  const [totpAvailable, web3Available, webauthnAvailable] = useMFAStore(
    (state) => [
      state.totp_available,
      state.web3_available,
      state.webauthn_available,
    ],
    shallow
  );

  const { mutate: mfaFinishMutation, isLoading: finishLoading } = useMutation(
    [MutationKeys.WEB3_MFA_FINISH],
    finish,
    {
      onSuccess: (data) => {
        const { user, url } = data;
        if (user && url) {
          resetMFAStore();
          setOpenIDStore({ openIDRedirect: true });
          window.location.replace(url);
          return;
        }
        if (user) {
          resetMFAStore();
          logIn(user);
        }
      },
      onError: (err) => {
        console.error(err);
        toaster.error(LL.loginPage.mfa.wallet.messages.walletErrorMfa());
        if (isConnected) {
        }
      },
    }
  );

  const { mutate: mfaStartMutation, isLoading: startLoading } = useMutation(
    [MutationKeys.WEB3_MFA_START],
    start,
    {
      onSuccess: (data) => {
        if (isConnected) {
          const message = JSON.parse(data.challenge);
          const types = message.types;
          const domain = message.domain;
          const value = message.message;
          signTypedData({ types, domain, value });
        } else {
          toaster.error(LL.loginPage.mfa.wallet.messages.walletError());
        }
      },
    }
  );

  const navigate = useNavigate();

  const { signTypedData, isLoading: isSigning } = useSignTypedData({
    onSuccess: (data) => {
      if (address) {
        mfaFinishMutation({
          address,
          signature: data,
        });
      }
    },
  });

  useEffect(() => {
    if (!web3Available) {
      navigate('../');
    }
  }, [navigate, web3Available]);

  const handleSigning = () => {
    if (address && isConnected) {
      mfaStartMutation({ address });
    }
  };

  return (
    <>
      <p>{LL.loginPage.mfa.wallet.header()}</p>
      <Button
        text={LL.loginPage.mfa.wallet.controls.submit()}
        styleVariant={ButtonStyleVariant.PRIMARY}
        size={ButtonSize.BIG}
        loading={finishLoading || startLoading || isConnecting || isSigning}
        onClick={() => {
          if (!isConnected) {
            setModalsState({
              connectWalletModal: { visible: true, onConnect: handleSigning },
            });
          } else {
            handleSigning();
          }
        }}
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
        {webauthnAvailable && (
          <Button
            text={LL.loginPage.mfa.controls.useWebauthn()}
            size={ButtonSize.BIG}
            onClick={() => navigate('../webauthn')}
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
