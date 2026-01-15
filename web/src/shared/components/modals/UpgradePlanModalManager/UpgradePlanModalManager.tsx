import { useEffect, useState } from 'react';
import {
  subscribeCloseModal,
  subscribeOpenModal,
} from '../../../hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../hooks/modalControls/modalTypes';
import type { OpenUpgradeLicenseModal } from '../../../hooks/modalControls/types';
import { ModalUpgradePlan } from '../ModalUpgradePlan/ModalUpgradePlan';

const modalNameKey = ModalName.UpgradeLicenseModal;

export const UpgradePlanModalManager = () => {
  const [modalData, setModalData] = useState<OpenUpgradeLicenseModal>({
    variant: 'business',
  });

  const [isOpen, setOpen] = useState(false);

  useEffect(() => {
    const openSub = subscribeOpenModal(modalNameKey, (data) => {
      setModalData(data);
      setOpen(true);
    });
    const closeSub = subscribeCloseModal(modalNameKey, () => setOpen(false));
    return () => {
      openSub.unsubscribe();
      closeSub.unsubscribe();
    };
  }, []);

  return (
    <ModalUpgradePlan
      variant={modalData.variant}
      isOpen={isOpen}
      onClose={() => {
        setOpen(false);
      }}
      afterClose={() => {
        setModalData({ variant: 'business' });
      }}
    />
  );
};
