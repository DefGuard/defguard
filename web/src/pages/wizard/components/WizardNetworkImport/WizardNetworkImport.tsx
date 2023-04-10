import './style.scss';

import { yupResolver } from '@hookform/resolvers/yup';
import { useMutation } from '@tanstack/react-query';
import { isUndefined } from 'lodash-es';
import { useCallback, useEffect, useMemo, useRef } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import * as yup from 'yup';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { FormInput } from '../../../../shared/components/Form/FormInput/FormInput';
import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../shared/components/layout/Button/Button';
import { Card } from '../../../../shared/components/layout/Card/Card';
import MessageBox from '../../../../shared/components/layout/MessageBox/MessageBox';
import useApi from '../../../../shared/hooks/useApi';
import { useToaster } from '../../../../shared/hooks/useToaster';
import { MutationKeys } from '../../../../shared/mutations';
import { ImportNetworkRequest } from '../../../../shared/types';
import { useWizardStore } from '../../hooks/useWizardStore';

interface FormInputs extends ImportNetworkRequest {
  fileName: string;
}
const defaultValues: FormInputs = {
  name: '',
  endpoint: '',
  fileName: '',
  config: '',
};
export const WizardNetworkImport = () => {
  const submitRef = useRef<HTMLInputElement>(null);
  const {
    network: { importNetwork },
  } = useApi();
  const toaster = useToaster();
  const [setWizardState, nextStep, submitSubject] = useWizardStore(
    (state) => [state.setState, state.nextStep, state.submitSubject],
    shallow
  );

  const { LL } = useI18nContext();

  const schema = useMemo(
    () =>
      yup
        .object({
          name: yup.string().required(LL.form.error.required()),
          endpoint: yup
            .string()
            .required(LL.form.error.required())
            .matches(
              // eslint-disable-next-line max-len
              /((25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)$/,
              LL.form.error.endpoint()
            ),
          fileName: yup.string().required(LL.form.error.required()),
          config: yup.string().required(),
        })
        .required(),
    [LL]
  );

  const { control, handleSubmit, setValue, setError, resetField } = useForm<FormInputs>({
    defaultValues,
    resolver: yupResolver(schema),
    mode: 'all',
    reValidateMode: 'onChange',
  });

  const {
    mutate: importNetworkMutation,
    isLoading,
    data,
  } = useMutation([MutationKeys.IMPORT_NETWORK], importNetwork, {
    onSuccess: async (response) => {
      toaster.success(LL.networkConfiguration.form.messages.networkCreated());
      setWizardState({
        importedNetworkDevices: response.devices,
        importedNetworkConfig: response.network,
      });
      nextStep();
    },
    onError: (err) => {
      toaster.error(LL.messages.error());
      resetField('fileName');
      resetField('config');
      console.error(err);
    },
  });

  const onValidSubmit: SubmitHandler<FormInputs> = useCallback(
    (data) => {
      if (!isLoading) {
        importNetworkMutation(data);
      }
    },
    [importNetworkMutation, isLoading]
  );

  const handleConfigUpload = () => {
    const input = document.createElement('input');
    input.type = 'file';
    input.multiple = false;
    input.style.display = 'none';
    input.onchange = () => {
      if (input.files && input.files.length === 1) {
        const reader = new FileReader();
        reader.onload = () => {
          if (reader.result && input.files) {
            const res = reader.result;
            setValue('config', res as string);
            setValue('fileName', input.files[0].name);
          }
        };
        reader.onerror = () => {
          toaster.error('Error while reading file.');
          setError('fileName', {
            message: 'Please try again',
          });
        };
        reader.readAsText(input.files[0]);
      }
    };
    input.click();
  };

  useEffect(() => {
    const sub = submitSubject.subscribe(() => {
      submitRef.current?.click();
    });
    return () => sub?.unsubscribe();
  }, [submitSubject]);

  return (
    <Card id="wizard-network-import">
      <form onSubmit={handleSubmit(onValidSubmit)}>
        <FormInput
          controller={{ control, name: 'name' }}
          outerLabel={LL.networkConfiguration.form.fields.name.label()}
          disabled={!isUndefined(data)}
        />
        <MessageBox>
          <p>{LL.networkConfiguration.form.messages.gateway()}</p>
        </MessageBox>
        <FormInput
          controller={{ control, name: 'endpoint' }}
          outerLabel={LL.networkConfiguration.form.fields.endpoint.label()}
          disabled={!isUndefined(data)}
        />
        <FormInput
          controller={{ control, name: 'fileName' }}
          outerLabel={LL.wizard.locations.form.fileName()}
          disabled
        />
        <Button
          text={LL.wizard.locations.form.selectFile()}
          size={ButtonSize.SMALL}
          styleVariant={ButtonStyleVariant.STANDARD}
          onClick={() => handleConfigUpload()}
          className="upload"
        />
        <input className="visually-hidden" type="submit" ref={submitRef} />
      </form>
    </Card>
  );
};
