import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { ModalWithTitle } from '../../../../../shared/defguard-ui/components/Layout/modals/ModalWithTitle/ModalWithTitle';
import { StartEnrollmentForm } from './components/StartEnrollmentForm';
import { useEnrollmentModalStore } from './hooks/useEnrollmentModalStore';

export const StartEnrollmentModal = () => {
  const { LL } = useI18nContext();
  const isOpen = useEnrollmentModalStore((state) => state.isOpen);

  const [close, reset] = useEnrollmentModalStore(
    (state) => [state.close, state.reset],
    shallow,
  );

  return (
    <ModalWithTitle
      id="start-enrollment-modal"
      backdrop
      title={LL.modals.startEnrollment.title()}
      isOpen={isOpen}
      onClose={() => close()}
      afterClose={() => reset()}
    >
      <StartEnrollmentForm />
    </ModalWithTitle>
  );
};
