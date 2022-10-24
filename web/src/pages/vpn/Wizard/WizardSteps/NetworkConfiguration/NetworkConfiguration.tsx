import './style.scss';

import { yupResolver } from '@hookform/resolvers/yup';
import { useEffect, useId, useRef } from 'react';
import { SubmitErrorHandler, SubmitHandler, useForm } from 'react-hook-form';
import { useTranslation } from 'react-i18next';
import useBreakpoint from 'use-breakpoint';
import * as yup from 'yup';
import shallow from 'zustand/shallow';

import { FormInput } from '../../../../../shared/components/Form/FormInput/FormInput';
import MessageBox, {
  MessageBoxType,
} from '../../../../../shared/components/layout/MessageBox/MessageBox';
import { deviceBreakpoints } from '../../../../../shared/constants';
import { useWizardStore } from '../store';

type Inputs = {
  networkIpAddress: string;
  networkEndpoint: string;
  networkPort: string;
  networkAllowedIps: string;
  networkDns: string;
};
interface Props {
  formId: number;
}
export const NetworkConfiguration = ({ formId }: Props) => {
  const { breakpoint } = useBreakpoint(deviceBreakpoints);
  const componentId = useId();
  const submitRef = useRef<HTMLInputElement>(null);
  const { t } = useTranslation('en');
  const [
    networkObserver,
    formSubmissionSubject,
    setFormStatus,
    proceedWizardSubject,
    setNetwork,
  ] = useWizardStore(
    (state) => [
      state.network,
      state.formSubmissionSubject,
      state.setFormStatus,
      state.proceedWizardSubject,
      state.setNetwork,
    ],
    shallow
  );
  const network = networkObserver ? networkObserver.getValue() : undefined;
  const schema = yup
    .object({
      networkIpAddress: yup
        .string()
        .required(t('wizard.networkIpAddress.validation.required'))
        .matches(
          /^(([0-9]|[1-9][0-9]|1[0-9]{2}|2[0-4][0-9]|25[0-5])\.){3}([0-9]|[1-9][0-9]|1[0-9]{2}|2[0-4][0-9]|25[0-5])(\/([1-9]|[12][0-9]|3[012])\b)?$/,
          t('wizard.networkIpAddress.validation.invalidAddress')
        ),
      networkEndpoint: yup
        .string()
        .required(t('wizard.networkEndpoint.validation.required'))
        .matches(
          /((25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)$/,
          t('wizard.networkEndpoint.validation.invalidEndpoint')
        ),
      networkPort: yup
        .number()
        .max(65535, t('form.errors.maximumLength', { length: 65535 }))
        .typeError(t('wizard.networkPort.validation.invalidPort'))
        .required(t('wizard.networkPort.validation.required')),
      networkAllowedIps: yup.string(),
      networkDns: yup.string(),
    })
    .required();

  const { handleSubmit, control } = useForm<Inputs>({
    resolver: yupResolver(schema),
    mode: 'all',
    defaultValues: {
      networkIpAddress: network?.address ?? '',
      networkEndpoint: network?.endpoint ?? '',
      networkDns: network?.dns ?? '',
      networkAllowedIps: network?.allowedIps ?? '',
      networkPort: String(network?.port ?? ''),
    },
  });

  const onValidSubmit: SubmitHandler<Inputs> = (data) => {
    setNetwork({
      address: data.networkIpAddress,
      port: Number(data.networkPort),
      allowedIps: data.networkAllowedIps,
      endpoint: data.networkEndpoint,
      dns: data.networkDns,
    });
    setFormStatus({ [formId]: true });
    proceedWizardSubject.next();
  };

  const onInvalidSubmit: SubmitErrorHandler<Inputs> = () => {
    setFormStatus({ [formId]: false });
  };

  useEffect(() => {
    const sub = formSubmissionSubject.subscribe((id) => {
      if (id === formId) {
        submitRef.current?.click();
      }
    });
    return () => sub.unsubscribe();
  }, [formId, formSubmissionSubject]);

  return (
    <div className="container-basic network-ip-address">
      {breakpoint !== 'desktop' && (
        <h1 className="step-name">Network configuration</h1>
      )}
      <form
        onSubmit={handleSubmit(onValidSubmit, onInvalidSubmit)}
        id={`${componentId}-form`}
      >
        <input type="submit" className="visually-hidden" ref={submitRef} />
        <div className="inputs-container">
          <FormInput
            controller={{ control, name: 'networkIpAddress' }}
            required
            placeholder={t('wizard.networkIpAddress.placeholder')}
            outerLabel={t('wizard.networkIpAddress.label')}
          />
          <FormInput
            controller={{ control, name: 'networkEndpoint' }}
            required
            placeholder={t('wizard.networkEndpoint.placeholder')}
            outerLabel={t('wizard.networkEndpoint.label')}
          />
          <MessageBox
            message={t('wizard.networkEndpoint.description')}
            type={MessageBoxType.INFO}
          />
          <FormInput
            controller={{ control, name: 'networkPort' }}
            required
            placeholder={t('wizard.networkPort.placeholder')}
            outerLabel={t('wizard.networkPort.label')}
          />
          <FormInput
            controller={{ control, name: 'networkAllowedIps' }}
            placeholder={t('wizard.networkAllowedIps.placeholder')}
            outerLabel={t('wizard.networkAllowedIps.label')}
          />
          <MessageBox
            message={t('wizard.networkAllowedIps.description')}
            type={MessageBoxType.INFO}
          />
          <FormInput
            controller={{ control, name: 'networkDns' }}
            placeholder={t('wizard.networkDns.placeholder')}
            outerLabel={t('wizard.networkDns.label')}
          />
          <MessageBox
            message={t('wizard.networkDns.description')}
            type={MessageBoxType.INFO}
          />
        </div>
      </form>
    </div>
  );
};
