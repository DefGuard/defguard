import { yupResolver } from '@hookform/resolvers/yup';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { isUndefined, omit } from 'lodash-es';
import { useEffect, useMemo, useState } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import * as yup from 'yup';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { FormInput } from '../../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { Button } from '../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../shared/defguard-ui/components/Layout/Button/types';
import { useModalStore } from '../../../../../shared/hooks/store/useModalStore';
import { useUserProfileStore } from '../../../../../shared/hooks/store/useUserProfileStore';
import useApi from '../../../../../shared/hooks/useApi';
import { MutationKeys } from '../../../../../shared/mutations';
import { QueryKeys } from '../../../../../shared/queries';
import { chainName } from '../../../../../shared/utils/chainName';
import { useWeb3Account } from '../../../../../shared/web3/hooks/useWeb3Account';
import { useWeb3Connection } from '../../../../../shared/web3/hooks/useWeb3Connection';
import { useWeb3Signer } from '../../../../../shared/web3/hooks/useWeb3Signer';

interface FormValues {
  name: string;
  address: string;
}

const defaultValues = {
  name: 'My wallet',
  address: '',
};

export const AddWalletModalForm = () => {
  const user = useUserProfileStore((state) => state.userProfile?.user);
  const setModalsState = useModalStore((state) => state.setState);
  const {
    user: { walletChallenge, setWallet },
  } = useApi();
  const { LL, locale } = useI18nContext();

  const queryClient = useQueryClient();

  const { isConnected } = useWeb3Connection();
  const { address, chainId } = useWeb3Account();
  const { signer } = useWeb3Signer();

  const [isSigning, setIsSigning] = useState(false);

  const AddWalletMutation = useMutation(setWallet, {
    mutationKey: [MutationKeys.SET_WALLET],

    onSuccess: () => {
      setModalsState({ addWalletModal: { visible: false } });
      queryClient.invalidateQueries([QueryKeys.FETCH_USER_PROFILE]);
    },

    onError: () => {
      setModalsState({ addWalletModal: { visible: false } });
    },
  });

  const WalletChallengeMutation = useMutation(walletChallenge, {
    mutationKey: [MutationKeys.WALLET_CHALLENGE],
    onSuccess: async (data, variables) => {
      if (isUndefined(chainId) || !signer) return;
      const message = JSON.parse(data.message);
      const types = omit(message.types, ['EIP712Domain']);
      const domain = message.domain;
      const value = message.message;
      setIsSigning(true);
      const signature = await signer.signTypedData(domain, types, value).catch((e) => {
        setIsSigning(false);
        console.error(e);
        return undefined;
      });
      if (signature) {
        AddWalletMutation.mutate({
          name: variables.name || 'My wallet',
          chain_id: chainId,
          username: variables.username,
          address: variables.address,
          signature,
        });
      }
    },
  });

  const schema = useMemo(() => {
    return yup
      .object({
        name: yup.string().required(LL.form.error.required()),
        address: yup.string().required(LL.form.error.required()),
      })
      .required();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [locale]);

  const defaultFormValues = useMemo((): FormValues => {
    if (address && chainId) {
      const mappedName = chainName(chainId);
      return {
        name: mappedName || 'My wallet',
        address: address || '',
      };
    }
    return defaultValues;
  }, [address, chainId]);

  const { handleSubmit, control, reset } = useForm<FormValues>({
    resolver: yupResolver(schema),
    mode: 'all',
    defaultValues: defaultFormValues,
  });

  const onSubmit: SubmitHandler<FormValues> = async (values) => {
    if (user?.username && chainId) {
      WalletChallengeMutation.mutate({
        name: values.name,
        username: user.username,
        address: values.address,
        chainId,
      });
    }
  };

  // makes sure default values are set upon mount
  useEffect(() => {
    reset();
  }, [defaultFormValues, reset]);

  return (
    <form onSubmit={handleSubmit(onSubmit, (e) => console.error(e))}>
      <FormInput
        controller={{ control, name: 'name' }}
        placeholder={LL.modals.addWallet.form.fields.name.placeholder()}
        label={LL.modals.addWallet.form.fields.name.label()}
      />
      <FormInput
        controller={{
          control,
          name: 'address',
        }}
        placeholder={LL.modals.addWallet.form.fields.address.placeholder()}
        label={LL.modals.addWallet.form.fields.address.label()}
        disabled={true}
      />
      <section className="controls">
        <Button
          size={ButtonSize.LARGE}
          text={LL.form.cancel()}
          className="cancel"
          onClick={async () => {
            setModalsState({ addWalletModal: { visible: false } });
          }}
          type="button"
        />
        {isConnected && (
          <Button
            size={ButtonSize.LARGE}
            styleVariant={ButtonStyleVariant.PRIMARY}
            type="submit"
            loading={isSigning || WalletChallengeMutation.isLoading}
            text={LL.modals.addWallet.form.controls.submit()}
          />
        )}
      </section>
    </form>
  );
};
