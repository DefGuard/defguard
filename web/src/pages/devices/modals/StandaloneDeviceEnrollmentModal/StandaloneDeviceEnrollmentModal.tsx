import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { Button } from '../../../../shared/defguard-ui/components/Layout/Button/Button';
import { ModalWithTitle } from '../../../../shared/defguard-ui/components/Layout/modals/ModalWithTitle/ModalWithTitle';
import { StandaloneDeviceModalEnrollmentContent } from '../components/StandaloneDeviceModalEnrollmentContent/StandaloneDeviceModalEnrollmentContent';
import { useStandaloneDeviceEnrollmentModal } from './store';

export const StandaloneDeviceEnrollmentModal = () => {
  const { LL } = useI18nContext();
  const localLL = LL.modals.standaloneDeviceEnrollmentModal;
  const [close, reset] = useStandaloneDeviceEnrollmentModal(
    (s) => [s.close, s.reset],
    shallow,
  );
  const isOpen = useStandaloneDeviceEnrollmentModal((s) => s.visible);
  return (
    <ModalWithTitle
      title={localLL.title()}
      isOpen={isOpen}
      onClose={close}
      afterClose={reset}
      includeDefaultStyles
    >
      <ModalContent />
    </ModalWithTitle>
  );
};

const ModalContent = () => {
  const { LL } = useI18nContext();
  const modalData = useStandaloneDeviceEnrollmentModal((s) => s.data);
  const closeModal = useStandaloneDeviceEnrollmentModal((s) => s.close, shallow);
  if (!modalData) return null;
  return (
    <>
      <StandaloneDeviceModalEnrollmentContent enrollmentData={modalData.enrollment} />
      <div className="controls solo">
        <Button
          text={LL.common.controls.close()}
          onClick={() => {
            closeModal();
          }}
        />
      </div>
    </>
  );
};
