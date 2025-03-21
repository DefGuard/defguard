import './style.scss';

import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { isNull, omit, omitBy } from 'lodash-es';
import { useEffect, useMemo, useRef, useState } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { z } from 'zod';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../i18n/i18n-react';
import { FormCheckBox } from '../../../shared/defguard-ui/components/Form/FormCheckBox/FormCheckBox.tsx';
import { FormInput } from '../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { FormSelect } from '../../../shared/defguard-ui/components/Form/FormSelect/FormSelect';
import { MessageBox } from '../../../shared/defguard-ui/components/Layout/MessageBox/MessageBox';
import { SelectOption } from '../../../shared/defguard-ui/components/Layout/Select/types';
import useApi from '../../../shared/hooks/useApi';
import { useToaster } from '../../../shared/hooks/useToaster';
import { QueryKeys } from '../../../shared/queries';
import { Network } from '../../../shared/types';
import { titleCase } from '../../../shared/utils/titleCase';
import { trimObjectStrings } from '../../../shared/utils/trimObjectStrings.ts';
import {
  validateIpList,
  validateIpOrDomain,
  validateIpOrDomainList,
} from '../../../shared/validators';
import { useNetworkPageStore } from '../hooks/useNetworkPageStore';

const aclDefaultPolicyOptions: SelectOption<boolean>[] = [
  {
    key: 'ALLOW',
    label: 'Allow',
    value: true,
  },
  {
    key: 'DENY',
    label: 'Deny',
    value: false,
  },
];

type FormFields = {
  address: string;
  endpoint: string;
  port: number;
  allowed_ips: string;
  allowed_groups: string[];
  name: string;
  dns: string;
  mfa_enabled: boolean;
  keepalive_interval: number;
  peer_disconnect_threshold: number;
  acl_enabled: boolean;
  acl_default_allow: boolean;
};

const defaultValues: FormFields = {
  address: '',
  endpoint: '',
  name: '',
  port: 50051,
  allowed_ips: '',
  allowed_groups: [],
  dns: '',
  mfa_enabled: false,
  keepalive_interval: 25,
  peer_disconnect_threshold: 180,
  acl_enabled: false,
  acl_default_allow: false,
};

const networkToForm = (data?: Network): FormFields => {
  if (!data) return defaultValues;
  let omited = omitBy<Network>(data, isNull);
  omited = omit(omited, [
    'id',
    'connected_at',
    'connected',
    'allowed_ips',
    'gateways',
    'address',
  ]);

  let allowed_ips = '';
  let address = '';

  if (Array.isArray(data.allowed_ips)) {
    allowed_ips = data.allowed_ips.join(',');
  }

  if (Array.isArray(data.address)) {
    address = data.address.join(',');
  }

  return { ...defaultValues, ...omited, allowed_ips, address };
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
        void queryClient.refetchQueries({
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

  const {
    error: groupsFetchError,
    isError: groupsError,
    isLoading: groupsLoading,
    data: groupsData,
  } = useQuery({
    queryKey: [QueryKeys.FETCH_GROUPS],
    queryFn: getGroups,
    enabled: componentMount,
    refetchOnMount: false,
    refetchOnWindowFocus: false,
    refetchOnReconnect: 'always',
  });

  useEffect(() => {
    if (groupsFetchError) {
      toaster.error(LL.messages.error());
      console.error(groupsFetchError);
    }
  }, [LL.messages, groupsFetchError, toaster]);

  useEffect(() => {
    if (groupsData) {
      setGroupOptions(
        groupsData.groups.map((g) => ({
          key: g,
          value: g,
          label: titleCase(g),
        })),
      );
    }
  }, [groupsData]);

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

  const zodSchema = useMemo(
    () =>
      z.object({
        name: z.string().min(1, LL.form.error.required()),
        address: z
          .string()
          .min(1, LL.form.error.required())
          .refine((value) => {
            return validateIpList(value, ',', true);
          }, LL.form.error.addressNetmask()),
        endpoint: z
          .string()
          .min(1, LL.form.error.required())
          .refine(
            (val) => validateIpOrDomain(val, false, true),
            LL.form.error.endpoint(),
          ),
        port: z
          .number({
            invalid_type_error: LL.form.error.required(),
          })
          .max(65535, LL.form.error.portMax()),
        allowed_ips: z.string(),
        dns: z
          .string()
          .optional()
          .refine((val) => {
            if (val === '' || !val) {
              return true;
            }
            return validateIpOrDomainList(val, ',', false, true);
          }, LL.form.error.allowedIps()),
        allowed_groups: z.array(z.string().min(1, LL.form.error.minimumLength())),
        mfa_enabled: z.boolean(),
        keepalive_interval: z
          .number({
            invalid_type_error: LL.form.error.required(),
          })
          .nonnegative()
          .min(1, LL.form.error.required()),
        peer_disconnect_threshold: z
          .number({
            invalid_type_error: LL.form.error.required(),
          })
          .min(120, LL.form.error.invalid()),
        acl_enabled: z.boolean(),
        acl_default_allow: z.boolean(),
      }),
    [LL.form.error],
  );

  const { control, handleSubmit, reset } = useForm<FormFields>({
    defaultValues: defaultFormValues,
    resolver: zodResolver(zodSchema),
    mode: 'all',
  });

  const onValidSubmit: SubmitHandler<FormFields> = (values) => {
    values = trimObjectStrings(values);
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
          type="number"
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
            const inf = options;
            return inf.filter((o) => o.value.toLowerCase().includes(val.toLowerCase()));
          }}
          renderSelected={(val) => ({
            key: val,
            displayValue: titleCase(val),
          })}
        />
        <FormCheckBox
          controller={{ control, name: 'mfa_enabled' }}
          label={LL.networkConfiguration.form.fields.mfa_enabled.label()}
          labelPlacement="right"
        />
        <FormCheckBox
          controller={{ control, name: 'acl_enabled' }}
          label={LL.networkConfiguration.form.fields.acl_enabled.label()}
          labelPlacement="right"
        />
        <FormSelect
          controller={{ control, name: 'acl_default_allow' }}
          label={LL.networkConfiguration.form.fields.acl_default_allow.label()}
          options={aclDefaultPolicyOptions}
          searchable={false}
        />
        <FormInput
          controller={{ control, name: 'keepalive_interval' }}
          label={LL.networkConfiguration.form.fields.keepalive_interval.label()}
          type="number"
        />
        <FormInput
          controller={{ control, name: 'peer_disconnect_threshold' }}
          label={LL.networkConfiguration.form.fields.peer_disconnect_threshold.label()}
          type="number"
        />
        <button type="submit" className="hidden" ref={submitRef}></button>
      </form>
    </section>
  );
};
