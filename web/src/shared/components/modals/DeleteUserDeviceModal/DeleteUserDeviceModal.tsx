import { useMutation } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import { m } from '../../../../paraglide/messages';
import api from '../../../api/api';
import { AppText } from '../../../defguard-ui/components/AppText/AppText';
import { Button } from '../../../defguard-ui/components/Button/Button';
import { Modal } from '../../../defguard-ui/components/Modal/Modal';
import { Snackbar } from '../../../defguard-ui/providers/snackbar/snackbar';
import { TextStyle } from '../../../defguard-ui/types';
import {
  subscribeCloseModal,
  subscribeOpenModal,
} from '../../../hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../hooks/modalControls/modalTypes';
import type { OpenDeleteUserDeviceModal } from '../../../hooks/modalControls/types';
import { Controls } from '../../Controls/Controls';

const modalNameValue = ModalName.DeleteUserDevice;

type ModalData = OpenDeleteUserDeviceModal;

export const DeleteUserDeviceModal = () => {
  const [isOpen, setOpen] = useState(false);
  const [modalData, setModalData] = useState<ModalData | null>(null);

  const { mutateAsync: deleteDevice, isPending } = useMutation({
    mutationFn: api.device.deleteDevice,
    meta: {
      invalidate: [['user-overview'], ['user'], ['network']],
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
      await deleteDevice(modalData.id);
      Snackbar.success(m.user_device_delete_success());
      setOpen(false);
    } catch {
      Snackbar.error(m.user_device_delete_failed());
    }
  };

  return (
    <Modal
      id="delete-user-device-modal"
      size="small"
      title={m.modal_delete_user_device_title()}
      isOpen={isOpen}
      onClose={() => setOpen(false)}
      afterClose={() => setModalData(null)}
    >
      <AppText font={TextStyle.TBodySm400}>
        {m.modal_delete_user_device_body({ name: modalData?.name ?? '' })}
      </AppText>
      <Controls>
        <div className="right">
          <Button
            text={m.controls_cancel()}
            variant="secondary"
            onClick={() => setOpen(false)}
            disabled={isPending}
          />
          <Button
            text={m.controls_delete()}
            variant="critical"
            testId="delete-user-device-confirm"
            onClick={handleDelete}
            loading={isPending}
            disabled={isPending}
          />
        </div>
      </Controls>
    </Modal>
  );
};
