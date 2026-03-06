import { useMutation } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import { m } from '../../../../paraglide/messages';
import api from '../../../../shared/api/api';
import { Controls } from '../../../../shared/components/Controls/Controls';
import { AppText } from '../../../../shared/defguard-ui/components/AppText/AppText';
import { Button } from '../../../../shared/defguard-ui/components/Button/Button';
import { Modal } from '../../../../shared/defguard-ui/components/Modal/Modal';
import { Snackbar } from '../../../../shared/defguard-ui/providers/snackbar/snackbar';
import { TextStyle } from '../../../../shared/defguard-ui/types';
import {
  subscribeCloseModal,
  subscribeOpenModal,
} from '../../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../../shared/hooks/modalControls/modalTypes';
import type { OpenDeleteAliasDestinationConfirmModal } from '../../../../shared/hooks/modalControls/types';

const modalNameValue = ModalName.DeleteAliasDestinationConfirm;

type ModalData = OpenDeleteAliasDestinationConfirmModal;

export const DeleteAliasDestinationConfirmModal = () => {
  const [isOpen, setOpen] = useState(false);
  const [modalData, setModalData] = useState<ModalData | null>(null);

  const { mutateAsync: deleteAlias, isPending: deleteAliasPending } = useMutation({
    mutationFn: api.acl.alias.deleteAlias,
    meta: {
      invalidate: ['acl', 'alias'],
    },
  });

  const { mutateAsync: deleteDestination, isPending: deleteDestinationPending } =
    useMutation({
      mutationFn: api.acl.destination.deleteDestination,
      meta: {
        invalidate: ['acl', 'destination'],
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
      if (modalData.target.kind === 'alias') {
        await deleteAlias(modalData.target.id);
        Snackbar.success(m.acl_alias_delete_success());
      } else {
        await deleteDestination(modalData.target.id);
        Snackbar.success(m.acl_destination_delete_success());
      }
      setOpen(false);
    } catch {
      if (modalData.target.kind === 'alias') {
        Snackbar.error(m.acl_alias_delete_failed());
      } else {
        Snackbar.error(m.acl_destination_delete_failed());
      }
    }
  };

  const isPending = deleteAliasPending || deleteDestinationPending;

  return (
    <Modal
      id="delete-alias-destination-confirm-modal"
      title={modalData?.title ?? ''}
      size="small"
      isOpen={isOpen}
      onClose={() => setOpen(false)}
      afterClose={() => setModalData(null)}
    >
      <AppText font={TextStyle.TBodySm400}>{modalData?.description ?? ''}</AppText>
      <Controls>
        <div className="right">
          <Button
            variant="secondary"
            text={m.controls_cancel()}
            onClick={() => setOpen(false)}
            disabled={isPending}
          />
          <Button
            variant="critical"
            text={m.controls_delete()}
            onClick={handleDelete}
            loading={isPending}
            disabled={isPending}
          />
        </div>
      </Controls>
    </Modal>
  );
};
