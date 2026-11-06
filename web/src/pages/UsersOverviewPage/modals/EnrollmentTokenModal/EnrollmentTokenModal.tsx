import { useEffect, useState } from 'react';
import { m } from '../../../../paraglide/messages';
import { Modal } from '../../../../shared/defguard-ui/components/Modal/Modal';
import { isPresent } from '../../../../shared/defguard-ui/utils/isPresent';
import {
  closeModal,
  subscribeCloseModal,
  subscribeOpenModal,
} from '../../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../../shared/hooks/modalControls/modalTypes';
import type { OpenEnrollmentTokenModal } from '../../../../shared/hooks/modalControls/types';
import { EnrollmentTokenContent } from '../../components/EnrollmentTokenContent/EnrollmentTokenContent';

const modalName = ModalName.SelfEnrollmentToken;

type ModalData = OpenEnrollmentTokenModal;

export const EnrollmentTokenModal = () => {
  const [isOpen, setOpen] = useState(false);
  const [modalData, setModalData] = useState<ModalData | null>(null);

  useEffect(() => {
    const openSub = subscribeOpenModal(modalName, (data) => {
      setModalData(data);
      setOpen(true);
    });
    const closeSub = subscribeCloseModal(modalName, () => setOpen(false));
    return () => {
      openSub.unsubscribe();
      closeSub.unsubscribe();
    };
  }, []);

  return (
    <Modal
      id="enrollment-token-modal"
      title={m.modal_initiate_self_enrollment_title()}
      isOpen={isOpen}
      onClose={() => setOpen(false)}
      afterClose={() => {
        setModalData(null);
      }}
    >
      {isPresent(modalData) && (
        <EnrollmentTokenContent {...modalData} onClose={() => closeModal(modalName)} />
      )}
    </Modal>
  );
};
