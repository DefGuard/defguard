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
        .required(),
    [LL.form.error]
  );
  const { handleSubmit, control } = useForm({
    mode: 'all',
    defaultValues: wizardNetworkConfiguration,
    resolver: yupResolver(schema),
  });

  const handleValidSubmit: SubmitHandler<ModifyNetworkRequest> = (values) => {
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
        <input type="submit" className="visually-hidden" ref={submitRef} />
      </form>
    </Card>
  );
};
