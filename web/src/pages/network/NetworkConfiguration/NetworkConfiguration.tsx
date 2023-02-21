import './style.scss';

import { yupResolver } from '@hookform/resolvers/yup';
import { useMutation, useQuery } from '@tanstack/react-query';
import { isNull, omit, omitBy } from 'lodash-es';
import { useEffect, useMemo, useRef } from 'react';
import { SubmitHandler, useFieldArray, useForm } from 'react-hook-form';
import { useQueryClient } from 'wagmi';
import * as yup from 'yup';

import { useI18nContext } from '../../../i18n/i18n-react';
import { FormInput } from '../../../shared/components/Form/FormInput/FormInput';
import { FormSelect } from '../../../shared/components/Form/FormSelect/FormSelect';
import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../shared/components/layout/Button/Button';
import { Card } from '../../../shared/components/layout/Card/Card';
import { Helper } from '../../../shared/components/layout/Helper/Helper';
import MessageBox from '../../../shared/components/layout/MessageBox/MessageBox';
import { SelectStyleVariant } from '../../../shared/components/layout/Select/Select';
import { IconArrowGrayUp } from '../../../shared/components/svg';
import useApi from '../../../shared/hooks/useApi';
import { useToaster } from '../../../shared/hooks/useToaster';
import { MutationKeys } from '../../../shared/mutations';
import { QueryKeys } from '../../../shared/queries';
import {
  Device,
  ModifyNetworkRequest,
  Network,
  SelectOption,
} from '../../../shared/types';
import { useNetworkPageStore } from '../hooks/useNetworkPageStore';

interface DeviceInput extends Omit<Device, 'user_id'> {
  user_id: SelectOption<number>;
}

interface FormInputs extends Omit<ModifyNetworkRequest, 'devices'> {
  devices: DeviceInput[];
}

const defaultValues: FormInputs = {
  address: '',
  endpoint: '',
  name: '',
  port: 50051,
  allowed_ips: '',
  dns: '',
  devices: [],
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
    network: { addNetwork, editNetwork, parseWireguardConfig },
  } = useApi();
  const submitRef = useRef<HTMLButtonElement | null>(null);
  const network = useNetworkPageStore((state) => state.network);
  const setStoreState = useNetworkPageStore((state) => state.setState);
  const submitSubject = useNetworkPageStore((state) => state.saveSubject);
  const queryClient = useQueryClient();
  const { LL } = useI18nContext();

  const { mutate: editNetworkMutation, isLoading: editLoading } = useMutation(
    [MutationKeys.CHANGE_NETWORK],
    editNetwork,
    {
      onSuccess: (network) => {
        setStoreState({ network, loading: false });
        toaster.success(
          LL.networkConfiguration.form.messages.networkModified()
        );
        queryClient.refetchQueries([QueryKeys.FETCH_NETWORK_TOKEN]);
      },
      onError: (err) => {
        setStoreState({ loading: false });
        toaster.error(LL.messages.error());
        console.error(err);
      },
    }
  );
  const { mutate: addNetworkMutation, isLoading: addLoading } = useMutation(
    [MutationKeys.ADD_NETWORK],
    addNetwork,
    {
      onSuccess: (network) => {
        setStoreState({ network, loading: false });
        toaster.success(LL.networkConfiguration.form.messages.networkCreated());
        queryClient.refetchQueries([QueryKeys.FETCH_NETWORK_TOKEN]);
      },
      onError: (err) => {
        setStoreState({ loading: false });
        toaster.error(LL.messages.error());
        console.error(err);
      },
    }
  );

  const schema = yup
    .object({
      name: yup.string().required(LL.form.error.required()),
      address: yup
        .string()
        .required(LL.form.error.required())
        .matches(
          /^(([0-9]|[1-9][0-9]|1[0-9]{2}|2[0-4][0-9]|25[0-5])\.){3}([0-9]|[1-9][0-9]|1[0-9]{2}|2[0-4][0-9]|25[0-5])(\/([1-9]|[12][0-9]|3[012])\b)?$/,
          LL.form.error.address()
        ),
      endpoint: yup
        .string()
        .required(LL.form.error.required())
        .matches(
          /((25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)$/,
          LL.form.error.endpoint()
        ),
      port: yup
        .number()
        .max(65535, LL.form.error.portMax())
        .typeError(LL.form.error.validPort())
        .required(LL.form.error.required()),
      allowed_ips: yup.string(),
      dns: yup.string(),
    })
    .required();

  const onValidSubmit: SubmitHandler<FormInputs> = (values) => {
    // Set device.user_id
    const devices: Device[] = values.devices?.map((d) => ({
      ...d,
      user_id: d.user_id.value,
    }));
    const result = { ...values, devices };
    if (network) {
      editNetworkMutation({ ...network, ...result });
    } else {
      addNetworkMutation(result);
    }
    setStoreState({ loading: true });
  };

  useEffect(() => {
    const sub = submitSubject.subscribe(() => submitRef.current?.click());
    return () => sub.unsubscribe();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const defaultFormValues = useMemo(() => {
    if (network) {
      const res = networkToForm(network);
      if (res) {
        return res;
      }
    }
    return defaultValues;
  }, [network]);

  const {
    control,
    handleSubmit,
    reset: resetForm,
  } = useForm<FormInputs>({
    defaultValues: defaultFormValues,
    resolver: yupResolver(schema),
  });
  const { fields, remove } = useFieldArray({
    control,
    name: 'devices',
  });
  const deviceToFormValues = (device: Device): DeviceInput => ({
    ...device,
    user_id: {
      label: users?.find((u) => u.id === device.user_id)?.username || '',
      value: device.user_id,
    },
  });
  const { mutate: parseConfigMutation, isLoading: parseLoading } = useMutation(
    [MutationKeys.PARSE_WIREGUARD_CONFIG],
    parseWireguardConfig,
    {
      onSuccess: (response) => {
        resetForm({
          ...networkToForm(response.network),
          ...{ devices: response.devices.map(deviceToFormValues) },
        });

        toaster.success(LL.networkConfiguration.form.messages.configParsed());
      },
      onError: (err) => {
        console.error(err);
        toaster.error(LL.messages.error());
      },
    }
  );

  useEffect(() => {
    setStoreState({ loading: addLoading || editLoading || parseLoading });
  }, [addLoading, editLoading, parseLoading, setStoreState]);

  // Displays file picker and posts selected file content to be parsed by API
  const parseConfig = () => {
    const input = document.createElement('input');
    input.type = 'file';
    input.onchange = () => {
      if (input.files == null) {
        return;
      }
      const reader = new FileReader();
      reader.addEventListener('loadend', () => {
        if (typeof reader.result === 'string') {
          parseConfigMutation(reader.result);
        }
      });
      reader.readAsText(input.files[0]);
    };
    input.click();
  };
  const {
    user: { getUsers },
  } = useApi();
  const { data: users, isLoading: usersLoading } = useQuery(
    [QueryKeys.FETCH_USERS],
    getUsers
  );

  const userOptions = useMemo(() => {
    if (!usersLoading && users) {
      return users.map((u) => ({
        key: u.id || -1,
        value: u.id || -1,
        label: u.username || '',
      }));
    }
    return [];
  }, [users, usersLoading]);

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
            <p>{LL.networkConfiguration.form.messages.gateway()}</p>
          </MessageBox>
          <FormInput
            controller={{ control, name: 'endpoint' }}
            outerLabel={LL.networkConfiguration.form.fields.endpoint.label()}
          />
          <FormInput
            controller={{ control, name: 'address' }}
            outerLabel={LL.networkConfiguration.form.fields.address.label()}
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
          <MessageBox>
            <p>{LL.networkConfiguration.form.messages.dns()}</p>
          </MessageBox>
          {!network &&
            fields.map((device, index) => (
              <div className="device-form" key={device.id}>
                <span>
                  <FormInput
                    controller={{ control, name: `devices.${index}.name` }}
                    outerLabel={LL.networkConfiguration.form.fields.deviceName.label()}
                  />
                </span>
                <span>
                  <FormSelect
                    styleVariant={SelectStyleVariant.WHITE}
                    options={userOptions}
                    controller={{ control, name: `devices.${index}.user_id` }}
                    outerLabel={LL.networkConfiguration.form.fields.deviceUser.label()}
                    loading={false}
                    searchable={false}
                    multi={false}
                    disabled={false}
                  />
                </span>
                <span className="wireguard-ip">{device.wireguard_ip}</span>{' '}
                <Button
                  text={LL.networkConfiguration.form.controls.remove()}
                  size={ButtonSize.SMALL}
                  styleVariant={ButtonStyleVariant.STANDARD}
                  onClick={() => remove(index)}
                />
              </div>
            ))}
          {!network && (
            <Button
              text={LL.networkConfiguration.form.controls.fill()}
              size={ButtonSize.SMALL}
              styleVariant={ButtonStyleVariant.STANDARD}
              icon={<IconArrowGrayUp />}
              loading={false}
              onClick={() => parseConfig()}
            />
          )}
          <button type="submit" className="hidden" ref={submitRef}></button>
        </form>
      </Card>
    </section>
  );
};
