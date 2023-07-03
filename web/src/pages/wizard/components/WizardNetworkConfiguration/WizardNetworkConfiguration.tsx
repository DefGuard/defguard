import './style.scss';

import { yupResolver } from '@hookform/resolvers/yup';
import { useEffect, useMemo, useRef } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { useMutation } from 'wagmi';
import * as yup from 'yup';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { FormInput } from '../../../../shared/components/Form/FormInput/FormInput';
import { Card } from '../../../../shared/components/layout/Card/Card';
import MessageBox from '../../../../shared/components/layout/MessageBox/MessageBox';
import useApi from '../../../../shared/hooks/useApi';
import { useToaster } from '../../../../shared/hooks/useToaster';
import { ModifyNetworkRequest } from '../../../../shared/types';
import {
  validateIp,
  validateIpList,
  validateIpOrDomain,
  validateIpOrDomainList,
} from '../../../../shared/validators';
import { useWizardStore } from '../../hooks/useWizardStore';

export const WizardNetworkConfiguration = () => {
  const submitRef = useRef<HTMLInputElement | null>(null);
  const {
    network: { addNetwork },
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
        })
        .required(),
    [LL.form.error]
  );
  const { handleSubmit, control } = useForm({
    mode: 'all',
    defaultValues: wizardNetworkConfiguration,
    resolver: yupResolver(schema),
  });

  const handleValidSubmit: SubmitHandler<ModifyNetworkRequest['network']> = (values) => {
    if (!isLoading) {
      setWizardState({ loading: true });
      addNetworkMutation(values);
    }
  };

  useEffect(() => {
    const sub = submitSubject.subscribe(() => {
      submitRef.current?.click();
    });
    return () => sub?.unsubscribe();
  }, [submitSubject]);

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
        <input type="submit" className="visually-hidden" ref={submitRef} />
      </form>
    </Card>
  );
};
