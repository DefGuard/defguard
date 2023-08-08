import { yupResolver } from '@hookform/resolvers/yup';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { useMemo } from 'react';
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

  const { address, isConnected } = useAccount();
  const { disconnect, disconnectAsync } = useDisconnect();
  const { chain } = useNetwork();
  const queryClient = useQueryClient();
  const { signTypedDataAsync } = useSignTypedData();

  const AddWalletMutation = useMutation(setWallet, {
    mutationKey: [MutationKeys.SET_WALLET],

    onSuccess: () => {
      setModalsState({ addWalletModal: { visible: false } });
      disconnect();
      queryClient.invalidateQueries([QueryKeys.FETCH_USER_PROFILE]);
    },

    onError: () => {
      setModalsState({ addWalletModal: { visible: false } });
      disconnect();
    },
  });

  const WalletChallengeMutation = useMutation(walletChallenge, {
    mutationKey: [MutationKeys.WALLET_CHALLENGE],
    onSuccess: async (data, variables) => {
      if (!chain?.id) return;
      const message = JSON.parse(data.message);
      const types = message.types;
      const domain = message.domain;
      const value = message.message;
      const signature = await signTypedDataAsync({ types, domain, value });
      AddWalletMutation.mutate({
        name: variables.name || 'My wallet',
        chain_id: chain.id,
        username: variables.username,
        address: variables.address,
        signature,
      });
    },
    onError: () => {
      disconnect();
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
    if (address && chain?.id) {
      const mappedName = chainName(chain.id);
      return {
        name: mappedName || 'My wallet',
        address: address || '',
      };
    }
    return defaultValues;
  }, [address, chain?.id]);

  const {
    handleSubmit,
    control,
    formState: { isValid, isSubmitted },
  } = useForm<FormValues>({
    resolver: yupResolver(schema),
    mode: 'all',
    defaultValues: defaultFormValues,
  });

  const onSubmit: SubmitHandler<FormValues> = async (values) => {
    const chainId = chain?.id;
    if (user?.username && chainId) {
      WalletChallengeMutation.mutate({
        name: values.name,
        username: user.username,
        address: values.address,
        chainId,
      });
    }
  };

  return (
    <form onSubmit={handleSubmit(onSubmit, (e) => console.error(e))}>
      <FormInput
        controller={{ control, name: 'name' }}
        placeholder={LL.modals.addWallet.form.fields.name.placeholder()}
        outerLabel={LL.modals.addWallet.form.fields.name.label()}
      />
      <FormInput
        controller={{
          control,
          name: 'address',
        }}
        placeholder={LL.modals.addWallet.form.fields.address.placeholder()}
        outerLabel={LL.modals.addWallet.form.fields.address.label()}
        disabled={true}
      />
      <section className="controls">
        <Button
          size={ButtonSize.LARGE}
          text={LL.form.cancel()}
          className="cancel"
          onClick={async () => {
            await disconnectAsync();
            setModalsState({ addWalletModal: { visible: false } });
          }}
          type="button"
        />
        {isConnected && (
          <Button
            size={ButtonSize.LARGE}
            styleVariant={ButtonStyleVariant.PRIMARY}
            type="submit"
            disabled={!isValid || isSubmitted}
            text={LL.modals.addWallet.form.controls.submit()}
          />
        )}
      </section>
    </form>
  );
};
