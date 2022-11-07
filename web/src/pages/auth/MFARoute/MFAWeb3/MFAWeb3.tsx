import { useMutation } from '@tanstack/react-query';
import { useEffect } from 'react';
import { useNavigate } from 'react-router';
import { useAccount, useDisconnect, useSignMessage } from 'wagmi';
import shallow from 'zustand/shallow';
import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../shared/components/layout/Button/Button';
import { useAuthStore } from '../../../../shared/hooks/store/useAuthStore';
import { useModalStore } from '../../../../shared/hooks/store/useModalStore';
import useApi from '../../../../shared/hooks/useApi';
import { MutationKeys } from '../../../../shared/mutations';
import { toaster } from '../../../../shared/utils/toaster';
import { useMFAStore } from '../../shared/hooks/useMFAStore';

export const MFAWeb3 = () => {
  const {
    auth: {
      mfa: {
        web3: { start, finish },
      },
    },
  } = useApi();

  const { isConnected, isConnecting, address } = useAccount();
  const { disconnect } = useDisconnect();
  const setModalsState = useModalStore((state) => state.setState);
  const logIn = useAuthStore((state) => state.logIn);
  const resetMFAStore = useMFAStore((state) => state.resetState);
  const [totpAvailable, web3Available, webauthnAvailable] = useMFAStore(
    (state) => [
      state.totp_available,
      state.web3_available,
      state.webautn_available,
    ],
    shallow
  );

  const { mutate: mfaFinishMutation, isLoading: finishLoading } = useMutation(
    [MutationKeys.WEB3_MFA_FINISH],
    finish,
    {
      onSuccess: (data) => {
        resetMFAStore();
        logIn(data);
      },
      onError: (err) => {
        console.error(err);
        toaster.error('Wallet is not authorized for MFA login.');
        if (isConnected) {
          disconnect();
        }
      },
    }
  );

  const {
    data: mfaMessage,
    mutate: mfaStartMutation,
    isLoading: startLoading,
  } = useMutation([MutationKeys.WEB3_MFA_START], start, {
    onSuccess: (data) => {
      signMessage({
        message: data.challenge,
      });
    },
  });

  const navigate = useNavigate();

  const { signMessage, isLoading: isSigning } = useSignMessage({
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
    mfaStartMutation();
    if (isConnected) {
      disconnect();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    if (!web3Available) {
      navigate('../');
    }
  }, [navigate, web3Available]);

  return (
    <>
      <p>
        Use your crypto wallet to sign in, please sign message in your wallet
        app or extension.
      </p>
      <Button
        text="Use your wallet"
        styleVariant={ButtonStyleVariant.PRIMARY}
        size={ButtonSize.BIG}
        loading={finishLoading || startLoading || isConnecting || isSigning}
        onClick={() => {
          if (!isConnected) {
            setModalsState({ connectWalletModal: { visible: true } });
          } else {
            if (mfaMessage?.challenge) {
              signMessage({
                message: mfaMessage.challenge,
              });
            }
          }
        }}
      />
      {webauthnAvailable || totpAvailable ? (
        <nav>
          <span>or</span>
          {totpAvailable && (
            <Button
              text="Use authenticator app instead"
              size={ButtonSize.BIG}
              onClick={() => navigate('../totp')}
            />
          )}
          {webauthnAvailable && (
            <Button
              text="Use security key insted"
              size={ButtonSize.BIG}
              onClick={() => navigate('../webauthn')}
            />
          )}
        </nav>
      ) : null}
    </>
  );
};
