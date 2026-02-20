import { useMutation } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import { m } from '../../../../paraglide/messages';
import api from '../../../../shared/api/api';
import { AppText } from '../../../../shared/defguard-ui/components/AppText/AppText';
import { Modal } from '../../../../shared/defguard-ui/components/Modal/Modal';
import { ModalControls } from '../../../../shared/defguard-ui/components/ModalControls/ModalControls';
import { Snackbar } from '../../../../shared/defguard-ui/providers/snackbar/snackbar';
import { TextStyle } from '../../../../shared/defguard-ui/types';
import {
  subscribeCloseModal,
  subscribeOpenModal,
} from '../../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../../shared/hooks/modalControls/modalTypes';
import type { OpenDeleteLocationModal } from '../../../../shared/hooks/modalControls/types';

const modalNameValue = ModalName.DeleteLocation;

type ModalData = OpenDeleteLocationModal;

export const DeleteLocationModal = () => {
  const [isOpen, setOpen] = useState(false);
  const [modalData, setModalData] = useState<ModalData | null>(null);

  const { mutateAsync: deleteLocation, isPending } = useMutation({
    mutationFn: api.location.deleteLocation,
    meta: {
      invalidate: [['network'], ['enterprise_info']],
    },
  });

  useEffect(() => {
    const openSub = subscribeOpenModal(modalNameValue, (data) => {
      setModalData(data);
      setOpen(true);
    });
    const closeSub = subscribeCloseModal(modalNameValue, () => setOpen(false));
    return () => {
      openSub.unsubscribe();
      closeSub.unsubscribe();
    };
  }, []);

  const handleDelete = async () => {
    if (!modalData) return;
    try {
      await deleteLocation(modalData.id);
      Snackbar.success(m.location_delete_success());
      setOpen(false);
    } catch {
      Snackbar.error(m.location_delete_failed());
    }
  };

  return (
    <Modal
      id="delete-location-modal"
      size="small"
      title={m.modal_delete_location_title()}
      isOpen={isOpen}
      onClose={() => setOpen(false)}
      afterClose={() => setModalData(null)}
    >
      <AppText font={TextStyle.TBodySm400}>
        {m.modal_delete_location_body({ name: modalData?.name ?? '' })}
      </AppText>
      <ModalControls
        submitProps={{
          text: m.controls_delete(),
          variant: 'critical',
          testId: 'delete-location-confirm',
          onClick: handleDelete,
          loading: isPending,
          disabled: isPending,
        }}
        cancelProps={{
          text: m.controls_cancel(),
          onClick: () => setOpen(false),
          disabled: isPending,
        }}
      />
    </Modal>
  );
};
