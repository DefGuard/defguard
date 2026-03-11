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
import type { OpenDeleteOpenIdClientModal } from '../../../../shared/hooks/modalControls/types';

const modalNameValue = ModalName.DeleteOpenIdClient;

type ModalData = OpenDeleteOpenIdClientModal;

export const DeleteOpenIdClientModal = () => {
  const [isOpen, setOpen] = useState(false);
  const [modalData, setModalData] = useState<ModalData | null>(null);

  const { mutateAsync: deleteClient, isPending } = useMutation({
    mutationFn: api.openIdClient.deleteOpenIdClient,
    meta: {
      invalidate: ['oauth'],
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
      await deleteClient(modalData.client_id);
      Snackbar.success(m.openid_delete_success());
      setOpen(false);
    } catch {
      Snackbar.error(m.openid_delete_failed());
    }
  };

  return (
    <Modal
      id="delete-openid-client-modal"
      size="small"
      title={m.modal_delete_openid_client_title()}
      isOpen={isOpen}
      onClose={() => setOpen(false)}
      afterClose={() => setModalData(null)}
    >
      <AppText font={TextStyle.TBodySm400}>{m.modal_delete_openid_client_body()}</AppText>
      <ModalControls
        submitProps={{
          text: m.controls_delete(),
          variant: 'critical',
          testId: 'delete-openid-client-confirm',
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
