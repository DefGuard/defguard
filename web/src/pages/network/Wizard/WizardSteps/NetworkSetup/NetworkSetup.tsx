import './style.scss';

import { useEffect, useRef } from 'react';
import { useI18nContext } from '../../../../../i18n/i18n-react';
import useBreakpoint from 'use-breakpoint';
import shallow from 'zustand/shallow';

import { deviceBreakpoints } from '../../../../../shared/constants';
import { useWizardStore } from '../store';
import { NetworkConfiguration } from '../../../NetworkConfiguration/NetworkConfiguration';
import { useNetworkPageStore } from '../../../hooks/useNetworkPageStore';

interface Props {
  formId: number;
}

export const NetworkSetup = ({ formId }: Props) => {
  const { breakpoint } = useBreakpoint(deviceBreakpoints);
  const submitRef = useRef<HTMLInputElement>(null);
  const formSubmissionSubject = useWizardStore(
    (state) => state.formSubmissionSubject
  );
  const { LL } = useI18nContext();
  const [save, loading] = useNetworkPageStore(
    (state) => [state.saveSubject, state.loading],
    shallow
  );

  useEffect(() => {
    const sub = formSubmissionSubject.subscribe((stepId) => {
      if (stepId === formId) {
        save.next();
      }
    });
    return () => sub.unsubscribe();
  }, [formId, formSubmissionSubject, save]);

  return (
    <>
      <div className="container-basic network-setup">
        {breakpoint !== 'desktop' && (
          <h1 className="step-name">{LL.wizard.wizardType.createNetwork()}</h1>
        )}
        <NetworkConfiguration />
        <input className="visually-hidden" type="submit" ref={submitRef} />
      </div>
    </>
  );
};
