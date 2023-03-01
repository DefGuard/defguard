import './style.scss';

import { yupResolver } from '@hookform/resolvers/yup';
import { useCallback, useEffect, useRef } from 'react';
import {
  Controller,
  SubmitErrorHandler,
  SubmitHandler,
  useForm,
} from 'react-hook-form';
import { useI18nContext } from '../../../../../i18n/i18n-react';
import useBreakpoint from 'use-breakpoint';
import * as yup from 'yup';
import shallow from 'zustand/shallow';

import { FormInput } from '../../../../../shared/components/Form/FormInput/FormInput';
import { deviceBreakpoints } from '../../../../../shared/constants';
import { useWizardStore } from '../store';
import MeshNetwork from './MeshNetwork';
import RegularNetwork from './RegularNetwork';

type Inputs = {
  name: string;
  type: inputNetworkType;
};

type inputNetworkType = 'mesh' | 'regular';

interface Props {
  formId: number;
}

export const NetworkType = ({ formId }: Props) => {
  const { breakpoint } = useBreakpoint(deviceBreakpoints);
  const submitRef = useRef<HTMLInputElement>(null);
  const formSubmissionSubject = useWizardStore(
    (state) => state.formSubmissionSubject
  );
  const [networkObserver, setNetwork, setFormStatus, proceedWizardSubject] =
    useWizardStore(
      (state) => [
        state.network,
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

  const network = networkObserver ? networkObserver.getValue() : undefined;

  const schema = yup
    .object({
      type: yup.mixed<inputNetworkType>().oneOf(['mesh', 'regular']).required(),
    })
    .required();

  const { handleSubmit, control } = useForm<Inputs>({
    resolver: yupResolver(schema),
    mode: 'all',
    defaultValues: {
      name: network?.name ?? '',
      type: network?.type ?? 'regular',
    },
  });

  useEffect(() => {
    const sub = formSubmissionSubject.subscribe((stepId) => {
      if (stepId === formId) {
        submitRef.current?.click();
      }
    });
    return () => sub.unsubscribe();
  }, [formId, formSubmissionSubject]);

  return (
    <>
      <div className="container-basic network-types">
        {breakpoint !== 'desktop' && (
          <h1 className="step-name">{LL.wizard.networkType.createNetwork()}</h1>
        )}
        <form onSubmit={handleSubmit(onValidSubmit, onInvalidSubmit)}>
          <Controller
            name="type"
            control={control}
            defaultValue={network?.type}
            render={({ field }) => (
              <div className="select-container">
                <MeshNetwork onChange={field.onChange} value={field.value} />
                <RegularNetwork onChange={field.onChange} value={field.value} />
              </div>
            )}
          />
          <input className="visually-hidden" type="submit" ref={submitRef} />
        </form>
      </div>
    </>
  );
};
