import './style.scss';

import { yupResolver } from '@hookform/resolvers/yup';
import { useMutation } from '@tanstack/react-query';
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
import useApi from '../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../shared/hooks/useToaster';
import { MutationKeys } from '../../../../../shared/mutations';
import { ImportNetworkRequest } from '../../../../../shared/types';
import { useWizardStore } from '../store';
import Button, { ButtonSize, ButtonStyleVariant } from '../../../../../shared/components/layout/Button/Button';
import { IconArrowGrayUp } from '../../../../../shared/components/svg';

// TODO: cleanup
// type inputNetworkType = 'mesh' | 'regular';

interface Props {
  formId: number;
}

interface FormInputs extends ImportNetworkRequest {
  fileName: string;
}

export const NetworkImport: React.FC<Props> = ({ formId }: Props) => {
  const { breakpoint } = useBreakpoint(deviceBreakpoints);
  const submitRef = useRef<HTMLInputElement>(null);
  const {
    network: { importNetwork },
  } = useApi();
  const toaster = useToaster();
  const [
    setNetwork,
    setState,
    setFormStatus,
    proceedWizardSubject,
    formSubmissionSubject,
  ] = useWizardStore(
    (state) => [
      state.setNetwork,
      state.setState,
      state.setFormStatus,
      state.proceedWizardSubject,
      state.formSubmissionSubject,
    ],
    shallow
  );
  const { mutateAsync: importNetworkMutation } = useMutation(
    [MutationKeys.IMPORT_NETWORK],
    importNetwork,
    {
      onSuccess: async (response) => {
        // TODO: cleanup
        console.log(response);
        setState({ devices: response.devices });
        toaster.success(LL.networkConfiguration.form.messages.networkCreated());
      },
      onError: (err) => {
        toaster.error(LL.messages.error());
        console.error(err);
      },
    }
  );
  const { LL } = useI18nContext();
  const onValidSubmit: SubmitHandler<FormInputs> = useCallback(
    async (data) => {
      // TODO: do we need that? maybe post straight away?
      setNetwork(data);
      await importNetworkMutation(data);
      setFormStatus({ [formId]: true });
      proceedWizardSubject.next();
    },
    [
      formId,
      proceedWizardSubject,
      setFormStatus,
      setNetwork,
      importNetworkMutation,
    ]
  );
  const onInvalidSubmit: SubmitErrorHandler<FormInputs> = () => {
    setFormStatus({ 2: false });
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
    fileName: '',
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
      config: yup.string().required(LL.form.error.required()),
    })
    .required();

  const { control, handleSubmit, reset, getValues } = useForm<FormInputs>({
    defaultValues,
    resolver: yupResolver(schema),
  });

  // Displays file picker and updates form with selected file
  const loadConfig = () => {
    const input = document.createElement('input');
    input.type = 'file';
    input.onchange = () => {
      const reader = new FileReader();
      reader.addEventListener('loadend', () => {
        if (typeof reader.result === 'string') {
          // parseConfigMutation(reader.result);
          console.log(reader.result);
          console.log(input.files);
          input.files?.[0] &&
            reset({
              ...getValues(),
              config: reader.result,
              fileName: input.files[0].name,
            });
        }
      });
      input.files?.[0] && reader.readAsText(input.files[0]);
    };
    input.click();
  };

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
              {/* TODO: hidden/disabled inputs */}
              <FormInput
                controller={{ control, name: 'fileName' }}
                disabled
                outerLabel={LL.wizard.TODO()}
              />
              <FormInput
                controller={{ control, name: 'config' }}
                disabled
                hidden
                outerLabel={LL.wizard.TODO()}
              />
              <Button
                text={LL.wizard.TODO()}
                size={ButtonSize.SMALL}
                styleVariant={ButtonStyleVariant.STANDARD}
                icon={<IconArrowGrayUp />}
                loading={false}
                onClick={() => loadConfig()}
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
