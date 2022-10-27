import { yupResolver } from '@hookform/resolvers/yup';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { useEffect, useMemo } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { useTranslation } from 'react-i18next';
import { toast } from 'react-toastify';
import { useAccount, useDisconnect, useNetwork, useSignMessage } from 'wagmi';
import * as yup from 'yup';
import shallow from 'zustand/shallow';

import { useModalStore } from '../../../hooks/store/useModalStore';
import useApi from '../../../hooks/useApi';
import { MutationKeys } from '../../../mutations';
import { QueryKeys } from '../../../queries';
import { chainName } from '../../../utils/chainName';
import { FormInput } from '../../Form/FormInput/FormInput';
import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../layout/Button/Button';
import MessageBox, { MessageBoxType } from '../../layout/MessageBox/MessageBox';
import ToastContent, { ToastType } from '../../Toasts/ToastContent';
import SignMessageLoader from './SignMessageLoader';
import WalletProviderList from './WalletProviderList';

interface FormValues {
  name: string;
  address: string;
}

const defaultValues = {
  name: 'My wallet',
  address: '',
};

const ChangeWalletForm: React.FC = () => {
  const [{ user }, setModalValues] = useModalStore(
    (state) => [state.changeWalletModal, state.setChangeWalletModal],
    shallow
  );

  const {
    user: { walletChallenge, setWallet },
  } = useApi();
  const { t } = useTranslation('en');

  const existingWallets = useMemo(() => {
    return user?.wallets?.map((wallet) => wallet.address) ?? [];
  }, [user?.wallets]);

  const { signMessageAsync } = useSignMessage();
  const { address, isConnected } = useAccount();
  const { disconnect } = useDisconnect();
  const { chain } = useNetwork();
  const queryClient = useQueryClient();

  const AddWalletMutation = useMutation(setWallet, {
    mutationKey: [MutationKeys.SET_WALLET],

    onSuccess: () => {
      setModalValues({ user: undefined, visible: false });
      disconnect();
      toast(<ToastContent type={ToastType.SUCCESS} message="Wallet added" />);
      queryClient.invalidateQueries([QueryKeys.FETCH_USER]);
    },

    onError: () => {
      setModalValues({ user: undefined, visible: false });
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

  const schema = yup
    .object({
      name: yup.string().required(t('form.errors.required')),
      address: yup
        .string()
        .required(t('form.errors.required'))
        .notOneOf(existingWallets, 'Address already exists'),
    })
    .required();

  const {
    handleSubmit,
    control,
    formState: { isValid, isSubmitted, errors },
    setError,
    setValue,
  } = useForm<FormValues>({
    resolver: yupResolver(schema),
    mode: 'all',
    defaultValues,
  });

  const getChainName = (chainId: number): string => {
    const chain = chainName(chainId);
    return chain || 'Unknown';
  };

  useEffect(() => {
    if (chain) {
      const name = `${getChainName(chain.id)} Wallet`;
      setValue('name', name);
    }
  }, [chain, setValue]);

  useEffect(() => {
    setValue(
      'address',
      address === null || address === undefined ? '' : address,
      { shouldValidate: true, shouldTouch: false, shouldDirty: true }
    );
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [address]);

  const onSubmit: SubmitHandler<FormValues> = async (values) => {
    const chainId = chain?.id;
    if (existingWallets.includes(values.address)) {
      setError('address', {
        type: 'custom',
        message: 'Address already exists',
      });
    } else if (user?.username && chainId) {
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
      <div className="form-content">
        {isConnected ? (
          isSubmitted && !Object.keys(errors).length ? (
            <SignMessageLoader />
          ) : (
            <div>
              <MessageBox type={MessageBoxType.INFO}>
                <p>
                  If you are using a mobile app to connect your wallet, check it
                  again to sign the message after pressing the &quot;Sign&quot;
                  button.
                </p>
              </MessageBox>
              <div style={{ marginTop: 20 }} />
              <div className="labeled-input">
                <label>Name:</label>
                <FormInput
                  controller={{ control, name: 'name' }}
                  placeholder="Wallet name"
                  tabIndex={2}
                />
              </div>
              <div className="labeled-input">
                <label>Address:</label>
                <FormInput
                  controller={{
                    control,
                    name: 'address',
                  }}
                  placeholder="Wallet address"
                  readOnly
                  tabIndex={2}
                />
              </div>
            </div>
          )
        ) : (
          <div>
            <MessageBox type={MessageBoxType.INFO}>
              <p>
                If you don&apos;t have a wallet yet, you can select a provider
                and create one now.
              </p>
            </MessageBox>
            <WalletProviderList />
          </div>
        )}
      </div>
      <section className="controls">
        <Button
          size={ButtonSize.BIG}
          text="Cancel"
          className="cancel"
          onClick={() => {
            setValue('address', '');
            setModalValues({ user: undefined, visible: false });
            if (isConnected) {
              disconnect();
            }
          }}
          tabIndex={4}
          type="button"
        />
        {isConnected && (
          <Button
            size={ButtonSize.BIG}
            styleVariant={ButtonStyleVariant.PRIMARY}
            type="submit"
            disabled={!isValid || isSubmitted}
            tabIndex={5}
            text="Sign"
          />
        )}
      </section>
    </form>
  );
};

export default ChangeWalletForm;
