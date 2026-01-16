import { useMutation } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import { m } from '../../../../../paraglide/messages';
import api from '../../../../../shared/api/api';
import type { ActivityLogStream } from '../../../../../shared/api/types';
import { AppText } from '../../../../../shared/defguard-ui/components/AppText/AppText';
import { Modal } from '../../../../../shared/defguard-ui/components/Modal/Modal';
import { ModalControls } from '../../../../../shared/defguard-ui/components/ModalControls/ModalControls';
import { TextStyle } from '../../../../../shared/defguard-ui/types';
import { ModalName } from '../../../../../shared/hooks/modalControls/modalTypes';
import {
  subscribeCloseModal,
  subscribeOpenModal,
} from '../../../../../shared/hooks/modalControls/modalsSubjects';

const modalNameValue = ModalName.DeleteLogStreaming;

type ModalData = ActivityLogStream;

export const DeleteLogStreamingModal = () => {
  const [isOpen, setOpen] = useState(false);
  const [modalData, setModalData] = useState<ModalData | null>(null);

  const { mutateAsync: deleteStream, isPending } = useMutation({
    mutationFn: api.activityLogStream.deleteStream,
    meta: {
      invalidate: ['activity_log_stream'],
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
    await deleteStream(modalData.id);
    setOpen(false);
  };

  return (
    <Modal
      id="delete-destination-modal"
      size="small"
      title={m.settings_activity_log_streaming_delete_log_streaming_title()}
      isOpen={isOpen}
      onClose={() => setOpen(false)}
      afterClose={() => setModalData(null)}
    >
      <AppText font={TextStyle.TBodySm400}>
        {m.modal_delete_logstream_destination()}
      </AppText>
      <ModalControls
        submitProps={{
          text: m.controls_delete(),
          variant: 'critical',
          testId: 'delete-destination-confirm',
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
