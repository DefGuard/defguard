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
import type { OpenDeleteGatewayModal } from '../../../../shared/hooks/modalControls/types';

const modalNameValue = ModalName.DeleteGateway;

type ModalData = OpenDeleteGatewayModal;

export const DeleteGatewayModal = () => {
  const [isOpen, setOpen] = useState(false);
  const [modalData, setModalData] = useState<ModalData | null>(null);

  const { mutateAsync: deleteGateway, isPending } = useMutation({
    mutationFn: api.gateway.deleteGateway,
    meta: {
      invalidate: ['gateway'],
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
      await deleteGateway(modalData.id);
      Snackbar.success(m.gateway_delete_success());
      setOpen(false);
    } catch {
      Snackbar.error(m.gateway_delete_failed());
    }
  };

  return (
    <Modal
      id="delete-gateway-modal"
      size="small"
      title={m.modal_delete_gateway_title()}
      isOpen={isOpen}
      onClose={() => setOpen(false)}
      afterClose={() => setModalData(null)}
    >
      <AppText font={TextStyle.TBodySm400}>
        {m.modal_delete_gateway_body({
          name: modalData?.name ?? '',
          locationName: modalData?.locationName ?? '',
        })}
      </AppText>
      <ModalControls
        submitProps={{
          text: m.controls_delete(),
          variant: 'critical',
          testId: 'delete-gateway-confirm',
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
