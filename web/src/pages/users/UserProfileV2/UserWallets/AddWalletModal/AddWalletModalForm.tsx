import { yupResolver } from '@hookform/resolvers/yup';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { useMemo } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { useTranslation } from 'react-i18next';
import { toast } from 'react-toastify';
import { useAccount, useDisconnect, useNetwork, useSignMessage } from 'wagmi';
import * as yup from 'yup';

import { FormInput } from '../../../../../shared/components/Form/FormInput/FormInput';
import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../shared/components/layout/Button/Button';
import ToastContent, {
  ToastType,
} from '../../../../../shared/components/Toasts/ToastContent';
import { useModalStore } from '../../../../../shared/hooks/store/useModalStore';
import { useUserProfileV2Store } from '../../../../../shared/hooks/store/useUserProfileV2Store';
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
  const user = useUserProfileV2Store((state) => state.user);
  const setModalsState = useModalStore((state) => state.setState);
  const {
    user: { walletChallenge, setWallet },
  } = useApi();
  const { t } = useTranslation('en');

  const { signMessageAsync } = useSignMessage();
  const { address, isConnected } = useAccount();
  const { disconnect, disconnectAsync } = useDisconnect();
  const { chain } = useNetwork();
  const queryClient = useQueryClient();

  const AddWalletMutation = useMutation(setWallet, {
    mutationKey: [MutationKeys.SET_WALLET],

    onSuccess: () => {
      setModalsState({ addWalletModal: { visible: false } });
      disconnect();
      toast(<ToastContent type={ToastType.SUCCESS} message="Wallet added" />);
      queryClient.invalidateQueries([QueryKeys.FETCH_USER]);
    },

    onError: () => {
      setModalsState({ addWalletModal: { visible: false } });
      disconnect();
      toast(
        <ToastContent
          type={ToastType.ERROR}
          message="Unexpected error occurred"
        />
      );
    },
  });

  const WalletChallengeMutation = useMutation(walletChallenge, {
    mutationKey: [MutationKeys.WALLET_CHALLENGE],
    onSuccess: async (data, variables) => {
      if (!chain?.id) return;

      const signature = await signMessageAsync({ message: data.message });

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
      toast(
        <ToastContent
          type={ToastType.ERROR}
          message="Unexpected error occurred"
        />
      );
    },
  });

  const schema = useMemo(() => {
    return yup
      .object({
        name: yup.string().required(t('form.errors.required')),
        address: yup.string().required(t('form.errors.required')),
      })
      .required();
  }, [t]);

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
        placeholder="Wallet name"
        outerLabel="Name"
      />
      <FormInput
        controller={{
          control,
          name: 'address',
        }}
        placeholder="Wallet address"
        outerLabel="Address"
        disabled={true}
      />
      <section className="controls">
        <Button
          size={ButtonSize.BIG}
          text="Cancel"
          className="cancel"
          onClick={async () => {
            await disconnectAsync();
            setModalsState({ addWalletModal: { visible: false } });
          }}
          type="button"
        />
        {isConnected && (
          <Button
            size={ButtonSize.BIG}
            styleVariant={ButtonStyleVariant.PRIMARY}
            type="submit"
            disabled={!isValid || isSubmitted}
            text="Add wallet"
          />
        )}
      </section>
    </form>
  );
};
