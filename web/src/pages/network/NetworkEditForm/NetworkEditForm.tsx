import './style.scss';

import { yupResolver } from '@hookform/resolvers/yup';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { isNull, omit, omitBy } from 'lodash-es';
import { useEffect, useMemo, useRef, useState } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import * as yup from 'yup';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../i18n/i18n-react';
import { FormInput } from '../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { FormSelect } from '../../../shared/defguard-ui/components/Form/FormSelect/FormSelect';
import { MessageBox } from '../../../shared/defguard-ui/components/Layout/MessageBox/MessageBox';
import { SelectOption } from '../../../shared/defguard-ui/components/Layout/Select/types';
import useApi from '../../../shared/hooks/useApi';
import { useToaster } from '../../../shared/hooks/useToaster';
import { QueryKeys } from '../../../shared/queries';
import { Network } from '../../../shared/types';
import { titleCase } from '../../../shared/utils/titleCase';
import {
  validateIp,
  validateIpList,
  validateIpOrDomain,
  validateIpOrDomainList,
} from '../../../shared/validators';
import { useNetworkPageStore } from '../hooks/useNetworkPageStore';

type FormFields = {
  address: string;
  endpoint: string;
  port: number;
  allowed_ips: string;
  allowed_groups: string[];
  name: string;
  dns: string;
};

const defaultValues: FormFields = {
  address: '',
  endpoint: '',
  name: '',
  port: 50051,
  allowed_ips: '',
  allowed_groups: [],
  dns: '',
};

const networkToForm = (data?: Network): FormFields => {
  if (!data) return defaultValues;
  let omited = omitBy<Network>(data, isNull);
  omited = omit(omited, [
    'id',
    'connected_at',
    'connected',
    'allowed_ips',
    'allowed_groups',
    'gateways',
  ]);

  let allowed_ips = '';
  const allowed_groups: string[] = [];

  if (Array.isArray(data.allowed_ips)) {
    allowed_ips = data.allowed_ips.join(',');
  }

  return { ...defaultValues, ...omited, allowed_groups, allowed_ips };
};

export const NetworkEditForm = () => {
  const toaster = useToaster();
  const {
    network: { editNetwork },
    groups: { getGroups },
  } = useApi();
  const submitRef = useRef<HTMLButtonElement | null>(null);
  const setStoreState = useNetworkPageStore((state) => state.setState);
  const submitSubject = useNetworkPageStore((state) => state.saveSubject);
  const [componentMount, setComponentMount] = useState(false);
  const [groupOptions, setGroupOptions] = useState<SelectOption<string>[]>([]);
  const [selectedNetworkId, networks] = useNetworkPageStore(
    (state) => [state.selectedNetworkId, state.networks],
    shallow,
  );
  const queryClient = useQueryClient();
  const { LL } = useI18nContext();

  const { mutate } = useMutation({
    mutationFn: editNetwork,
    onSuccess: () => {
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
    },
    onError: (err) => {
      setStoreState({ loading: false });
      console.error(err);
      toaster.error(LL.messages.error());
    },
  });

  const { isError: groupsError, isLoading: groupsLoading } = useQuery({
    queryKey: [QueryKeys.FETCH_GROUPS],
    queryFn: getGroups,
    onSuccess: (res) => {
      setGroupOptions(
        res.groups.map((g) => ({
          key: g,
          value: g,
          label: titleCase(g),
        })),
      );
    },
    onError: (err) => {
      toaster.error(LL.messages.error());
      console.error(err);
    },
    enabled: componentMount,
    refetchOnMount: false,
    refetchOnWindowFocus: false,
    refetchOnReconnect: 'always',
  });

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

  const { control, handleSubmit, reset } = useForm<FormFields>({
    defaultValues: defaultFormValues,
    resolver: yupResolver(schema),
    mode: 'all',
  });

  const onValidSubmit: SubmitHandler<FormFields> = async (values) => {
    setStoreState({ loading: true });
    mutate({
      id: selectedNetworkId,
      network: {
        ...values,
      },
    });
  };

  // reset form when network is selected
  useEffect(() => {
    reset(defaultFormValues);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [defaultFormValues, reset]);

  useEffect(() => {
    setTimeout(() => setComponentMount(true), 100);
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
          label={LL.networkConfiguration.form.fields.name.label()}
        />
        <MessageBox>
          <p>{LL.networkConfiguration.form.helpers.address()}</p>
        </MessageBox>
        <FormInput
          controller={{ control, name: 'address' }}
          label={LL.networkConfiguration.form.fields.address.label()}
        />
        <MessageBox>
          <p>{LL.networkConfiguration.form.helpers.gateway()}</p>
        </MessageBox>
        <FormInput
          controller={{ control, name: 'endpoint' }}
          label={LL.networkConfiguration.form.fields.endpoint.label()}
        />
        <FormInput
          controller={{ control, name: 'port' }}
          label={LL.networkConfiguration.form.fields.port.label()}
        />
        <MessageBox>
          <p>{LL.networkConfiguration.form.helpers.allowedIps()}</p>
        </MessageBox>
        <FormInput
          controller={{ control, name: 'allowed_ips' }}
          label={LL.networkConfiguration.form.fields.allowedIps.label()}
        />
        <MessageBox>
          <p>{LL.networkConfiguration.form.helpers.dns()}</p>
        </MessageBox>
        <FormInput
          controller={{ control, name: 'dns' }}
          label={LL.networkConfiguration.form.fields.dns.label()}
        />
        <MessageBox>
          <p>{LL.networkConfiguration.form.helpers.allowedGroups()}</p>
        </MessageBox>
        <FormSelect
          controller={{ control, name: 'allowed_groups' }}
          label={LL.networkConfiguration.form.fields.allowedGroups.label()}
          loading={groupsLoading}
          disabled={groupsError || (!groupsLoading && groupOptions.length === 0)}
          options={groupOptions}
          placeholder={LL.networkConfiguration.form.fields.allowedGroups.placeholder()}
          searchable
          searchFilter={(val, options) => {
            const inf = options as SelectOption<string>[];
            return inf.filter((o) => o.value.toLowerCase().includes(val.toLowerCase()));
          }}
          renderSelected={(val) => ({
            key: val,
            displayValue: titleCase(val),
          })}
        />
        <button type="submit" className="hidden" ref={submitRef}></button>
      </form>
    </section>
  );
};
