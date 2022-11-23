import './style.scss';

import { yupResolver } from '@hookform/resolvers/yup';
import { useMutation } from '@tanstack/react-query';
import { isNull, omit, omitBy } from 'lodash-es';
import { useEffect, useMemo, useRef } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import * as yup from 'yup';

import { FormInput } from '../../../shared/components/Form/FormInput/FormInput';
import { Card } from '../../../shared/components/layout/Card/Card';
import { Helper } from '../../../shared/components/layout/Helper/Helper';
import MessageBox from '../../../shared/components/layout/MessageBox/MessageBox';
import useApi from '../../../shared/hooks/useApi';
import { useToaster } from '../../../shared/hooks/useToaster';
import { MutationKeys } from '../../../shared/mutations';
import { ModifyNetworkRequest, Network } from '../../../shared/types';
import { useNetworkPageStore } from '../hooks/useNetworkPageStore';

const schema = yup
  .object({
    name: yup.string().required('Field is required'),
    address: yup
      .string()
      .required('Field is required')
      .matches(
        /^(([0-9]|[1-9][0-9]|1[0-9]{2}|2[0-4][0-9]|25[0-5])\.){3}([0-9]|[1-9][0-9]|1[0-9]{2}|2[0-4][0-9]|25[0-5])(\/([1-9]|[12][0-9]|3[012])\b)?$/,
        'Enter a valid address'
      ),
    endpoint: yup
      .string()
      .required('Field is required')
      .matches(
        /((25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)$/,
        'Enter a valid endpoint'
      ),
    port: yup
      .number()
      .max(65535, 'Maximum port is 65535')
      .typeError('Enter valid port')
      .required('Field is required'),
    allowed_ips: yup.string(),
    dns: yup.string(),
  })
  .required();

type FormInputs = ModifyNetworkRequest;

const defaultValues: FormInputs = {
  address: '',
  endpoint: '',
  name: '',
  port: 0,
  allowed_ips: '',
  dns: '',
};

const networkToForm = (data?: Network): FormInputs | undefined => {
  if (!data) return undefined;
  const omited = omitBy(omit(data, ['id', 'connected_at']), isNull);
  if (Array.isArray(omited.allowed_ips)) {
    omited.allowed_ips = omited.allowed_ips.join(',');
  }
  return { ...defaultValues, ...omited } as FormInputs;
};

export const NetworkConfiguration = () => {
  const toaster = useToaster();
  const {
    network: { addNetwork, editNetwork },
  } = useApi();
  const submitRef = useRef<HTMLButtonElement | null>(null);
  const network = useNetworkPageStore((state) => state.network);
  const setStoreState = useNetworkPageStore((state) => state.setState);
  const submitSubject = useNetworkPageStore((state) => state.saveSubject);
  const { mutate: editNetworkMutation, isLoading: editLoading } = useMutation(
    [MutationKeys.CHANGE_NETWORK],
    editNetwork,
    {
      onSuccess: (response) => {
        setStoreState({ network: response });
        toaster.success('Network modified');
      },
      onError: (err) => {
        console.error(err);
        toaster.error('Unexpected error occurred.');
      },
    }
  );
  const { mutate: addNetworkMutation, isLoading: addLoading } = useMutation(
    [MutationKeys.ADD_NETWORK],
    addNetwork,
    {
      onSuccess: (network) => {
        setStoreState({ network, loading: false });
        toaster.success('Network added');
      },
      onError: (err) => {
        setStoreState({ loading: false });
        toaster.error('Unexpected error occurred.');
        console.error(err);
      },
    }
  );

  const defaultFormValues = useMemo(() => {
    if (network) {
      const res = networkToForm(network);
      if (res) {
        return res;
      }
    }
    return defaultValues;
  }, [network]);

  const { control, handleSubmit } = useForm<FormInputs>({
    defaultValues: defaultFormValues,
    resolver: yupResolver(schema),
  });

  const onValidSubmit: SubmitHandler<FormInputs> = (values) => {
    if (network) {
      editNetworkMutation({ ...network, ...values });
    } else {
      addNetworkMutation(values);
    }
    setStoreState({ loading: true });
  };

  useEffect(() => {
    setStoreState({ loading: addLoading || editLoading });
  }, [addLoading, editLoading, setStoreState]);

  useEffect(() => {
    const sub = submitSubject.subscribe(() => submitRef.current?.click());
    return () => sub.unsubscribe();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <section className="network-config">
      <header>
        <h2>Network configuration</h2>
        <Helper>
          <p>PLACEHOLDER</p>
        </Helper>
      </header>
      <Card>
        <form onSubmit={handleSubmit(onValidSubmit)}>
          <FormInput
            controller={{ control, name: 'name' }}
            outerLabel="Network name"
          />
          <FormInput
            controller={{ control, name: 'address' }}
            outerLabel="VPN network address and mask"
          />
          <MessageBox>
            <p>Gateway{"'"}s public address, used by VPN users to connect</p>
          </MessageBox>
          <FormInput
            controller={{ control, name: 'endpoint' }}
            outerLabel="Gateway address"
          />
          <FormInput
            controller={{ control, name: 'port' }}
            outerLabel="Gateway port"
          />
          <MessageBox>
            <p>
              List of addresses/masks that should be routed through the VPN
              network
            </p>
          </MessageBox>
          <FormInput
            controller={{ control, name: 'allowed_ips' }}
            outerLabel="Allowed Ips"
          />
          <FormInput controller={{ control, name: 'dns' }} outerLabel="DNS" />
          <MessageBox>
            <p>
              Specify the DNS resolvers to query when the WireGuard interface is
              up.
            </p>
          </MessageBox>
          <button type="submit" className="hidden" ref={submitRef}></button>
        </form>
      </Card>
    </section>
  );
};
