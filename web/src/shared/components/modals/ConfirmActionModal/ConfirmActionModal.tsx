import './style.scss';
import { useMutation } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import { m } from '../../../../paraglide/messages';
import { Button } from '../../../defguard-ui/components/Button/Button';
import { Modal } from '../../../defguard-ui/components/Modal/Modal';
import { RenderMarkdown } from '../../../defguard-ui/components/RenderMarkdown/RenderMarkdown';
import { isPresent } from '../../../defguard-ui/utils/isPresent';
import {
  closeModal,
  subscribeCloseModal,
  subscribeOpenModal,
} from '../../../hooks/modalControls/modalsSubjects';
import type { ModalNameValue } from '../../../hooks/modalControls/modalTypes';
import type { OpenConfirmActionModal } from '../../../hooks/modalControls/types';
import { Controls } from '../../Controls/Controls';

const modalNameValue: ModalNameValue = 'confirmAction';

type ModalData = OpenConfirmActionModal;

export const ConfirmActionModal = () => {
  const [isOpen, setOpen] = useState(false);
  const [modalData, setModalData] = useState<ModalData | null>(null);

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
  return (
    <Modal
      id="action-modal"
      size="small"
      title={modalData?.title ?? ''}
      isOpen={isOpen}
      onClose={() => {
        setOpen(false);
      }}
      afterClose={() => {
        setModalData(null);
      }}
    >
      {isPresent(modalData) && <ModalContent data={modalData} />}
    </Modal>
  );
};

const ModalContent = ({ data }: { data: ModalData }) => {
  const { mutate, isPending } = useMutation({
    mutationFn: data.actionPromise,
    meta: {
      invalidate: data.invalidateKeys,
    },
    onSuccess: () => {
      closeModal(modalNameValue);
      data.onSuccess?.();
    },
    onError: (e) => {
      data.onError?.();
      console.error(e);
    },
  });

  return (
    <>
      <RenderMarkdown
        containerProps={data.contentContainerProps}
        content={data.contentMd}
      />
      <Controls>
        <div className="right">
          <Button
            text={m.controls_cancel()}
            variant="secondary"
            {...data.cancelProps}
            disabled={isPending}
            onClick={() => {
              closeModal(modalNameValue);
            }}
          />
          <Button
            text={m.controls_submit()}
            variant="primary"
            {...data.submitProps}
            loading={isPending}
            onClick={() => {
              mutate();
            }}
          />
        </div>
      </Controls>
    </>
  );
};
