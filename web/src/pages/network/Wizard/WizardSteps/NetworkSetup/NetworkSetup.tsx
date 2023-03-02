import './style.scss';

import { yupResolver } from '@hookform/resolvers/yup';
import { useCallback, useEffect, useRef } from 'react';
import { SubmitErrorHandler, SubmitHandler, useForm } from 'react-hook-form';
import { useI18nContext } from '../../../../../i18n/i18n-react';
import useBreakpoint from 'use-breakpoint';
import * as yup from 'yup';
import shallow from 'zustand/shallow';

import { deviceBreakpoints } from '../../../../../shared/constants';
import { useWizardStore } from '../store';
import { NetworkConfiguration } from '../../../NetworkConfiguration/NetworkConfiguration';
import { useNetworkPageStore } from '../../../hooks/useNetworkPageStore';
import { useNavigate } from 'react-router';

type Inputs = {
  name: string;
  type: inputNetworkType;
};

type inputNetworkType = 'mesh' | 'regular';

interface Props {
  formId: number;
}

export const NetworkSetup = ({ formId }: Props) => {
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
  const navigate = useNavigate();

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

  const [save, loading] = useNetworkPageStore(
    (state) => [state.saveSubject, state.loading],
    shallow
  );

  useEffect(() => {
    const sub = formSubmissionSubject.subscribe((stepId) => {
      if (stepId === formId) {
        save.next();
        // submitRef.current?.click();
      }
    });
    return () => sub.unsubscribe();
  }, [formId, formSubmissionSubject, save]);

  return (
    <>
      <div className="container-basic network-setup">
        {breakpoint !== 'desktop' && (
          <h1 className="step-name">{LL.wizard.networkType.createNetwork()}</h1>
        )}
        <NetworkConfiguration />
        <input className="visually-hidden" type="submit" ref={submitRef} />
      </div>
    </>
  );
};
