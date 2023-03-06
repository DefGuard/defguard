import './style.scss';

import { yupResolver } from '@hookform/resolvers/yup';
import { useCallback, useEffect, useRef } from 'react';
import { SubmitErrorHandler, SubmitHandler, useForm } from 'react-hook-form';
import useBreakpoint from 'use-breakpoint';
import * as yup from 'yup';
import shallow from 'zustand/shallow';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { FormInput } from '../../../../../shared/components/Form/FormInput/FormInput';
import { Card } from '../../../../../shared/components/layout/Card/Card';
import { Helper } from '../../../../../shared/components/layout/Helper/Helper';
import MessageBox from '../../../../../shared/components/layout/MessageBox/MessageBox';
import { deviceBreakpoints } from '../../../../../shared/constants';
import { ImportNetworkRequest } from '../../../../../shared/types';
import { useWizardStore } from '../store';

type Inputs = {
  name: string;
  endpoint: string;
  config: string;
};

// type inputNetworkType = 'mesh' | 'regular';

interface Props {
  formId: number;
}

type FormInputs = ImportNetworkRequest;

export const NetworkImport: React.FC<Props> = ({ formId }: Props) => {
  const { breakpoint } = useBreakpoint(deviceBreakpoints);
  const submitRef = useRef<HTMLInputElement>(null);
  const formSubmissionSubject = useWizardStore(
    (state) => state.formSubmissionSubject
  );
  const [setNetwork, setFormStatus, proceedWizardSubject] = useWizardStore(
    (state) => [
      state.setNetwork,
      state.setFormStatus,
      state.proceedWizardSubject,
    ],
    shallow
  );
  const { LL } = useI18nContext();
  const onValidSubmit: SubmitHandler<Inputs> = useCallback(
    (data) => {
      setNetwork(data);
      setFormStatus({ [formId]: true });
      proceedWizardSubject.next();
    },
    [formId, proceedWizardSubject, setFormStatus, setNetwork]
  );
  const onInvalidSubmit: SubmitErrorHandler<Inputs> = () => {
    setFormStatus({ 1: false });
  };

  // TODO: cleanup
  // const network = networkObserver ? networkObserver.getValue() : undefined;

  // const schema = yup
  //   .object({
  //     type: yup.mixed<inputNetworkType>().oneOf(['mesh', 'regular']).required(),
  //   })
  //   .required();

  // const { handleSubmit, control } = useForm<Inputs>({
  //   resolver: yupResolver(schema),
  //   mode: 'all',
  //   defaultValues: {
  //     name: network?.name ?? '',
  //     type: network?.type ?? 'regular',
  //   },
  // });

  // TODO: use loading?
  // const [save, loading] = useNetworkPageStore(
  //   (state) => [state.saveSubject, state.loading],
  //   shallow
  // );

  useEffect(() => {
    const sub = formSubmissionSubject.subscribe((stepId) => {
      if (stepId === formId) {
        // TODO: cleanup
        // save.next();
        submitRef.current?.click();
      }
    });
    return () => sub.unsubscribe();
    // }, [formId, formSubmissionSubject, save]);
  }, [formId, formSubmissionSubject]);

  const defaultValues: FormInputs = {
    name: '',
    endpoint: '',
    config: '',
  };

  const schema = yup
    .object({
      name: yup.string().required(LL.form.error.required()),
      endpoint: yup
        .string()
        .required(LL.form.error.required())
        .matches(
          /((25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)$/,
          LL.form.error.endpoint()
        ),
    })
    .required();

  const { control, handleSubmit } = useForm<FormInputs>({
    defaultValues,
    resolver: yupResolver(schema),
  });

  return (
    <>
      <div className="container-basic network-setup">
        {breakpoint !== 'desktop' && (
          <h1 className="step-name">{LL.wizard.networkType.createNetwork()}</h1>
        )}
        <section className="network-config">
          <header>
            <h2>{LL.networkConfiguration.importHeader()}</h2>
            <Helper>
              <p>PLACEHOLDER</p>
            </Helper>
          </header>
          <Card>
            <form onSubmit={handleSubmit(onValidSubmit, onInvalidSubmit)}>
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
              {/* TODO: config select button */}
              <FormInput
                controller={{ control, name: 'config' }}
                outerLabel={LL.networkConfiguration.form.fields.address.label()}
              />
              <input
                className="visually-hidden"
                type="submit"
                ref={submitRef}
              />
            </form>
          </Card>
        </section>
      </div>
    </>
  );
};
