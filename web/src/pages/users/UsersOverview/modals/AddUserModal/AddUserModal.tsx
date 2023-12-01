import './style.scss';

import { ReactNode, useMemo } from 'react';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { ModalWithTitle } from '../../../../../shared/defguard-ui/components/Layout/modals/ModalWithTitle/ModalWithTitle';
import { AddUserForm } from './components/AddUserForm/AddUserForm';
import { EnrollmentTokenCard } from './components/EnrollmentTokenCard/EnrollmentTokenCard';
import { StartEnrollmentForm } from './components/StartEnrollmentForm/StartEnrollmentForm';
import { useAddUserModal } from './hooks/useAddUserModal';

const steps: ReactNode[] = [
  <AddUserForm key={0} />,
  <StartEnrollmentForm key={1} />,
  <EnrollmentTokenCard key={2} />,
];

export const AddUserModal = () => {
  const { LL } = useI18nContext();

  const [currentStep, visible, desktop] = useAddUserModal(
    (state) => [state.step, state.visible, state.desktop],
    shallow,
  );

  const [reset, close] = useAddUserModal((state) => [state.reset, state.close], shallow);

  const getTitle = useMemo(() => {
    if (desktop) {
      return LL.modals.startEnrollment.desktopTitle();
    }
    return currentStep === 0
      ? LL.modals.addUser.title()
      : LL.modals.startEnrollment.title();
  }, [LL.modals.addUser, LL.modals.startEnrollment, desktop, currentStep]);

  return (
    <ModalWithTitle
      id="add-user-modal"
      backdrop
      title={getTitle}
      onClose={close}
      afterClose={reset}
      steps={steps}
      currentStep={currentStep}
      isOpen={visible}
    />
  );
};
