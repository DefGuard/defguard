import { useMutation } from '@tanstack/react-query';
import { isUndefined, omit } from 'lodash-es';
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
import { useWeb3Account } from '../../../../shared/web3/hooks/useWeb3Account';
import { useWeb3Connection } from '../../../../shared/web3/hooks/useWeb3Connection';
import { useWeb3Signer } from '../../../../shared/web3/hooks/useWeb3Signer';
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

  const { signer } = useWeb3Signer();
  const { address } = useWeb3Account();
  const { isConnected, connect } = useWeb3Connection();
  const toaster = useToaster();
  const [isSigning, setIsSigning] = useState(false);
  const [connecting, setConnecting] = useState(false);
  const web3Available = useMFAStore((state) => state.web3_available);
  const loginSubject = useAuthStore((state) => state.loginSubject);

  const { mutate: mfaFinishMutation, isLoading: finishLoading } = useMutation(
    [MutationKeys.WEB3_MFA_FINISH],
    finish,
    {
      onSuccess: (data) => loginSubject.next(data),
      onError: (err) => {
        console.error(err);
        toaster.error(LL.loginPage.mfa.wallet.messages.walletErrorMfa());
      },
    },
  );

  const { mutate: mfaStartMutation, isLoading: startLoading } = useMutation(
    [MutationKeys.WEB3_MFA_START],
    start,
    {
      onSuccess: async (data) => {
        if (isConnected && signer && address) {
          const message = JSON.parse(data.challenge);
          const types = omit(message.types, ['EIP712Domain']);
          const domain = message.domain;
          const value = message.message;
          signer
            .signTypedData(domain, types, value)
            .then((signature: string) => {
              setIsSigning(false);
              mfaFinishMutation({
                address,
                signature,
              });
            })
            .catch((e) => {
              setIsSigning(false);
              toaster.error(LL.loginPage.mfa.wallet.messages.walletError());
              console.error(e);
            });
        } else {
          toaster.error(LL.loginPage.mfa.wallet.messages.walletError());
        }
      },
    },
  );

  const navigate = useNavigate();

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
        size={ButtonSize.LARGE}
        disabled={isUndefined(connect) || isUndefined(window.ethereum)}
        loading={finishLoading || startLoading || isSigning || connecting}
        onClick={() => {
          if (!isConnected && connect) {
            connect()
              .then(({ address }) => {
                setConnecting(false);
                mfaStartMutation({
                  address,
                });
              })
              .catch((e) => {
                setConnecting(false);
                console.error(e);
              });
          } else {
            handleSigning();
          }
        }}
      />
    </>
  );
};
