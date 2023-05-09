import './style.scss';

import { yupResolver } from '@hookform/resolvers/yup';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { isNull, omit, omitBy } from 'lodash-es';
import { useEffect, useMemo, useRef } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { useNavigate } from 'react-router';
import * as yup from 'yup';

import { useI18nContext } from '../../../i18n/i18n-react';
import { FormInput } from '../../../shared/components/Form/FormInput/FormInput';
import { Card } from '../../../shared/components/layout/Card/Card';
import { Helper } from '../../../shared/components/layout/Helper/Helper';
import MessageBox from '../../../shared/components/layout/MessageBox/MessageBox';
import useApi from '../../../shared/hooks/useApi';
import { useToaster } from '../../../shared/hooks/useToaster';
import { MutationKeys } from '../../../shared/mutations';
import { QueryKeys } from '../../../shared/queries';
import { ModifyNetworkRequest, Network } from '../../../shared/types';
import {
  validateIp,
  validateIpList,
  validateIpOrDomain,
  validateIpOrDomainList,
} from '../../../shared/validators';
import { useNetworkPageStore } from '../hooks/useNetworkPageStore';

type FormInputs = ModifyNetworkRequest;

const defaultValues: FormInputs = {
  address: '',
  endpoint: '',
  name: '',
  port: 50051,
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

export const NetworkConfiguration: React.FC = () => {
  const toaster = useToaster();
  const {
    network: { addNetwork, editNetwork },
  } = useApi();
  const submitRef = useRef<HTMLButtonElement | null>(null);
  const network = useNetworkPageStore((state) => state.network);
  const setStoreState = useNetworkPageStore((state) => state.setState);
  const submitSubject = useNetworkPageStore((state) => state.saveSubject);
  const queryClient = useQueryClient();
  const { LL } = useI18nContext();

  const { mutateAsync: editNetworkMutation, isLoading: editLoading } = useMutation(
    [MutationKeys.CHANGE_NETWORK],
    editNetwork,
    {
      onSuccess: async (response) => {
        setStoreState({ network: response });
        toaster.success(LL.networkConfiguration.form.messages.networkModified());
        await queryClient.refetchQueries([QueryKeys.FETCH_NETWORK_TOKEN]);
      },
      onError: (err) => {
        console.error(err);
        toaster.error(LL.messages.error());
      },
    }
  );
  const { mutateAsync: addNetworkMutation, isLoading: addLoading } = useMutation(
    [MutationKeys.ADD_NETWORK],
    addNetwork,
    {
      onSuccess: async (network) => {
        setStoreState({ network, loading: false });
        toaster.success(LL.networkConfiguration.form.messages.networkCreated());
        await queryClient.refetchQueries([QueryKeys.FETCH_NETWORK_TOKEN]);
      },
      onError: (err) => {
        setStoreState({ loading: false });
        toaster.error(LL.messages.error());
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

  const schema = yup
    .object({
      name: yup.string().required(LL.form.error.required()),
      address: yup
        .string()
        .required(LL.form.error.required())
        .test(LL.form.error.address(), (value: string) => {
          const ipValid = validateIp(value, true);
          if (ipValid) {
            const host = value.split('.')[3].split('/')[0];
            if (host === '0') return false;
          }
          return ipValid;
        }),
      endpoint: yup
        .string()
        .required(LL.form.error.required())
        .test(LL.form.error.endpoint(), (val: string) => validateIpOrDomain(val)),
      port: yup
        .number()
        .max(65535, LL.form.error.portMax())
        .typeError(LL.form.error.validPort())
        .required(LL.form.error.required()),
      allowed_ips: yup
        .string()
        .optional()
        .test(LL.form.error.allowedIps(), (val?: string) => {
          if (val === '' || !val) {
            return true;
          }
          return validateIpList(val, ',', true);
        }),
      dns: yup
        .string()
        .optional()
        .test(LL.form.error.allowedIps(), (val?: string) => {
          if (val === '' || !val) {
            return true;
          }
          return validateIpOrDomainList(val, ',', true);
        }),
    })
    .required();

  const { control, handleSubmit } = useForm<FormInputs>({
    defaultValues: defaultFormValues,
    resolver: yupResolver(schema),
    mode: 'all',
  });

  const navigate = useNavigate();
  const onValidSubmit: SubmitHandler<FormInputs> = async (values) => {
    setStoreState({ loading: true });
    if (network) {
      await editNetworkMutation({ ...network, ...values });
    } else {
      await addNetworkMutation(values);
    }
    navigate('/admin/network');
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
        <h2>{LL.networkConfiguration.header()}</h2>
        <Helper>
          <p>PLACEHOLDER</p>
        </Helper>
      </header>
      <Card>
        <form onSubmit={handleSubmit(onValidSubmit)}>
          <FormInput
            controller={{ control, name: 'name' }}
            outerLabel={LL.networkConfiguration.form.fields.name.label()}
          />
          <MessageBox>
            <p>{LL.networkConfiguration.form.messages.address()}</p>
          </MessageBox>
          <FormInput
            controller={{ control, name: 'address' }}
            outerLabel={LL.networkConfiguration.form.fields.address.label()}
          />
          <MessageBox>
            <p>{LL.networkConfiguration.form.messages.gateway()}</p>
          </MessageBox>
          <FormInput
            controller={{ control, name: 'endpoint' }}
            outerLabel={LL.networkConfiguration.form.fields.endpoint.label()}
          />
          <FormInput
            controller={{ control, name: 'port' }}
            outerLabel={LL.networkConfiguration.form.fields.port.label()}
          />
          <MessageBox>
            <p>{LL.networkConfiguration.form.messages.allowedIps()}</p>
          </MessageBox>
          <FormInput
            controller={{ control, name: 'allowed_ips' }}
            outerLabel={LL.networkConfiguration.form.fields.allowedIps.label()}
          />
          <MessageBox>
            <p>{LL.networkConfiguration.form.messages.dns()}</p>
          </MessageBox>
          <FormInput
            controller={{ control, name: 'dns' }}
            outerLabel={LL.networkConfiguration.form.fields.dns.label()}
          />
          <button type="submit" className="hidden" ref={submitRef}></button>
        </form>
      </Card>
    </section>
  );
};
