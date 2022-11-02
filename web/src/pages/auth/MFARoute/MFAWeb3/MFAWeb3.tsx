import { useMutation } from '@tanstack/react-query';
import { useEffect } from 'react';
import { useAccount, useSignMessage } from 'wagmi';

import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../shared/components/layout/Button/Button';
import { useAuthStore } from '../../../../shared/hooks/store/useAuthStore';
import { useModalStore } from '../../../../shared/hooks/store/useModalStore';
import useApi from '../../../../shared/hooks/useApi';
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

  const { isConnected, isConnecting, address } = useAccount();
  const setModalsState = useModalStore((state) => state.setState);
  const logIn = useAuthStore((state) => state.logIn);
  const resetMFAStore = useMFAStore((state) => state.resetState);

  const { mutate: mfaFinishMutation, isLoading: finishLoading } = useMutation(
    [MutationKeys.WEB3_MFA_FINISH],
    finish,
    {
      onSuccess: (data) => {
        resetMFAStore();
        logIn(data);
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
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

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
      <div className="mfa-methods"></div>
      <nav>
        <span>or</span>
        <Button text="Use authenticator app instead" />
        <Button text="Use security key insted" />
      </nav>
    </>
  );
};
