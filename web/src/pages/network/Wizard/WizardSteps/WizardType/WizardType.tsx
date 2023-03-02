import './style.scss';

import { yupResolver } from '@hookform/resolvers/yup';
import { useCallback, useEffect, useRef } from 'react';
import {
  Controller,
  SubmitErrorHandler,
  SubmitHandler,
  useForm,
} from 'react-hook-form';
import useBreakpoint from 'use-breakpoint';
import * as yup from 'yup';
import shallow from 'zustand/shallow';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { deviceBreakpoints } from '../../../../../shared/constants';
import { useWizardStore } from '../store';
import MeshNetwork from './MeshNetwork';
import RegularNetwork from './RegularNetwork';

type Inputs = {
  name: string;
  type: inputNetworkType;
};

type inputNetworkType = 'regular' | 'import';

interface Props {
  formId: number;
}

export const WizardType = ({ formId }: Props) => {
  const { breakpoint } = useBreakpoint(deviceBreakpoints);
  const submitRef = useRef<HTMLInputElement>(null);
  const formSubmissionSubject = useWizardStore(
    (state) => state.formSubmissionSubject
  );
  const [setFormStatus, proceedWizardSubject, setState, type] = useWizardStore(
    (state) => {
      return [
        state.setFormStatus,
        state.proceedWizardSubject,
        state.setState,
        state.type,
      ];
    },
    shallow
  );
  const { LL } = useI18nContext();
  const onValidSubmit: SubmitHandler<Inputs> = useCallback(
    (data) => {
      setState({ type: data.type });
      setFormStatus({ [formId]: true });
      proceedWizardSubject.next();
    },
    [formId, proceedWizardSubject, setFormStatus, setState]
  );
  const onInvalidSubmit: SubmitErrorHandler<Inputs> = () => {
    setFormStatus({ 1: false });
  };

  const schema = yup
    .object({
      type: yup
        .mixed<inputNetworkType>()
        .oneOf(['import', 'regular'])
        .required(),
    })
    .required();

  const { handleSubmit, control } = useForm<Inputs>({
    resolver: yupResolver(schema),
    mode: 'all',
    defaultValues: {},
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
            defaultValue={type}
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
