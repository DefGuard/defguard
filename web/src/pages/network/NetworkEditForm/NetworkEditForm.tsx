import './style.scss';

import { yupResolver } from '@hookform/resolvers/yup';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { isNull, omit, omitBy } from 'lodash-es';
import { useEffect, useMemo, useRef } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import * as yup from 'yup';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../i18n/i18n-react';
import { FormInput } from '../../../shared/components/Form/FormInput/FormInput';
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

type FormInputs = ModifyNetworkRequest['network'];

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

export const NetworkEditForm = () => {
  const toaster = useToaster();
  const {
    network: { editNetwork },
  } = useApi();
  const submitRef = useRef<HTMLButtonElement | null>(null);
  const setStoreState = useNetworkPageStore((state) => state.setState);
  const submitSubject = useNetworkPageStore((state) => state.saveSubject);
  const [selectedNetworkId, networks] = useNetworkPageStore(
    (state) => [state.selectedNetworkId, state.networks],
    shallow
  );
  const queryClient = useQueryClient();
  const { LL } = useI18nContext();

  const { mutateAsync } = useMutation([MutationKeys.CHANGE_NETWORK], editNetwork);

  const defaultFormValues = useMemo(() => {
    if (selectedNetworkId && networks) {
      const network = networks.find((n) => n.id === selectedNetworkId);
      if (network) {
        const res = networkToForm(network);
        if (res) {
          return res;
        }
      }
    }
    return defaultValues;
  }, [networks, selectedNetworkId]);

  const schema = yup
    .object({
      name: yup.string().required(LL.form.error.required()),
      address: yup
        .string()
        .required(LL.form.error.required())
        .test(LL.form.error.address(), (value: string) => {
          const netmaskPresent = value.split('/').length == 2;
          if (!netmaskPresent) {
            return false;
          }
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

  const { control, handleSubmit, reset } = useForm<FormInputs>({
    defaultValues: defaultFormValues,
    resolver: yupResolver(schema),
    mode: 'all',
  });

  const onValidSubmit: SubmitHandler<FormInputs> = async (values) => {
    setStoreState({ loading: true });
    mutateAsync({
      id: selectedNetworkId,
      network: values,
    })
      .then(() => {
        setStoreState({ loading: false });
        toaster.success(LL.networkConfiguration.form.messages.networkModified());
        const keys = [
          QueryKeys.FETCH_NETWORK,
          QueryKeys.FETCH_NETWORKS,
          QueryKeys.FETCH_NETWORK_TOKEN,
        ];
        for (const key of keys) {
          queryClient.refetchQueries({
            queryKey: [key],
          });
        }
      })
      .catch((err) => {
        setStoreState({ loading: false });
        console.error(err);
        toaster.error(LL.messages.error());
      });
  };

  // reset form when network is selected
  useEffect(() => {
    reset(defaultFormValues);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [defaultFormValues, reset]);

  useEffect(() => {
    const sub = submitSubject.subscribe(() => submitRef.current?.click());
    return () => sub.unsubscribe();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <section className="network-config">
      <header>
        <h2>{LL.networkConfiguration.header()}</h2>
      </header>
      <form onSubmit={handleSubmit(onValidSubmit)}>
        <FormInput
          controller={{ control, name: 'name' }}
          outerLabel={LL.networkConfiguration.form.fields.name.label()}
        />
        <MessageBox>
          <p>{LL.networkConfiguration.form.helpers.address()}</p>
        </MessageBox>
        <FormInput
          controller={{ control, name: 'address' }}
          outerLabel={LL.networkConfiguration.form.fields.address.label()}
        />
        <MessageBox>
          <p>{LL.networkConfiguration.form.helpers.gateway()}</p>
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
          <p>{LL.networkConfiguration.form.helpers.allowedIps()}</p>
        </MessageBox>
        <FormInput
          controller={{ control, name: 'allowed_ips' }}
          outerLabel={LL.networkConfiguration.form.fields.allowedIps.label()}
        />
        <MessageBox>
          <p>{LL.networkConfiguration.form.helpers.dns()}</p>
        </MessageBox>
        <FormInput
          controller={{ control, name: 'dns' }}
          outerLabel={LL.networkConfiguration.form.fields.dns.label()}
        />
        <button type="submit" className="hidden" ref={submitRef}></button>
      </form>
    </section>
  );
};
