import './style.scss';

import { yupResolver } from '@hookform/resolvers/yup';
import { useMutation, useQuery } from '@tanstack/react-query';
import { useEffect, useMemo, useRef, useState } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import * as yup from 'yup';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { FormInput } from '../../../../shared/components/Form/FormInput/FormInput';
import { FormSelect } from '../../../../shared/components/Form/FormSelect/FormSelect';
import { Card } from '../../../../shared/components/layout/Card/Card';
import MessageBox from '../../../../shared/components/layout/MessageBox/MessageBox';
import {
  SelectOption,
  SelectStyleVariant,
} from '../../../../shared/components/layout/Select/Select';
import useApi from '../../../../shared/hooks/useApi';
import { useToaster } from '../../../../shared/hooks/useToaster';
import { QueryKeys } from '../../../../shared/queries';
import { ModifyNetworkRequest } from '../../../../shared/types';
import { titleCase } from '../../../../shared/utils/titleCase';
import {
  validateIp,
  validateIpList,
  validateIpOrDomain,
  validateIpOrDomainList,
} from '../../../../shared/validators';
import { useWizardStore } from '../../hooks/useWizardStore';

type FormInputs = Omit<ModifyNetworkRequest['network'], 'allowed_groups'> & {
  allowed_groups: SelectOption<string>[];
};

export const WizardNetworkConfiguration = () => {
  const [componentMount, setComponentMount] = useState(false);
  const [groupOptions, setGroupOptions] = useState<SelectOption<string>[]>([]);
  const submitRef = useRef<HTMLInputElement | null>(null);
  const {
    network: { addNetwork },
    groups: { getGroups },
  } = useApi();

  const [submitSubject, nextSubject, setWizardState] = useWizardStore(
    (state) => [state.submitSubject, state.nextStepSubject, state.setState],
    shallow
  );

  const wizardNetworkConfiguration = useWizardStore((state) => state.manualNetworkConfig);

  const toaster = useToaster();
  const { LL } = useI18nContext();

  const { mutate: addNetworkMutation, isLoading } = useMutation(addNetwork, {
    onSuccess: () => {
      setWizardState({ loading: false });
      toaster.success(LL.wizard.configuration.successMessage());
      nextSubject.next();
    },
    onError: (err) => {
      setWizardState({ loading: false });
      toaster.error(LL.messages.error());
      console.error(err);
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
        }))
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

  const schema = useMemo(
    () =>
      yup
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
          allowed_groups: yup.array().optional(),
        })
        .required(),
    [LL.form.error]
  );

  const getDefaultValues = useMemo((): FormInputs => {
    return { ...wizardNetworkConfiguration, allowed_groups: [] };
  }, [wizardNetworkConfiguration]);

  const { handleSubmit, control } = useForm<FormInputs>({
    mode: 'all',
    defaultValues: getDefaultValues,
    resolver: yupResolver(schema),
  });

  const handleValidSubmit: SubmitHandler<FormInputs> = (values) => {
    if (!isLoading) {
      setWizardState({ loading: true });
      addNetworkMutation({
        ...values,
        allowed_groups: values.allowed_groups.map((o) => o.value),
      });
    }
  };

  useEffect(() => {
    const sub = submitSubject.subscribe(() => {
      submitRef.current?.click();
    });
    return () => sub?.unsubscribe();
  }, [submitSubject]);

  useEffect(() => {
    setTimeout(() => setComponentMount(true), 100);
  }, []);

  return (
    <Card id="wizard-manual-network-configuration" shaded>
      <form onSubmit={handleSubmit(handleValidSubmit)}>
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
        <MessageBox>
          <p>{LL.networkConfiguration.form.helpers.allowedGroups()}</p>
        </MessageBox>
        <FormSelect
          styleVariant={SelectStyleVariant.WHITE}
          controller={{ control, name: 'allowed_groups' }}
          outerLabel={LL.networkConfiguration.form.fields.allowedGroups.label()}
          loading={groupsLoading}
          disabled={groupsError || (!groupsLoading && groupOptions.length === 0)}
          options={groupOptions}
          placeholder={LL.networkConfiguration.form.fields.allowedGroups.placeholder()}
          multi
          searchable
        />
        <input type="submit" className="visually-hidden" ref={submitRef} />
      </form>
    </Card>
  );
};
