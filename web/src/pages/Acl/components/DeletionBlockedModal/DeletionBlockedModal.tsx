import { useEffect, useState } from 'react';
import { m } from '../../../../paraglide/messages';
import { Controls } from '../../../../shared/components/Controls/Controls';
import { AppText } from '../../../../shared/defguard-ui/components/AppText/AppText';
import { Button } from '../../../../shared/defguard-ui/components/Button/Button';
import { Modal } from '../../../../shared/defguard-ui/components/Modal/Modal';
import { TextStyle } from '../../../../shared/defguard-ui/types';
import {
  subscribeCloseModal,
  subscribeOpenModal,
} from '../../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../../shared/hooks/modalControls/modalTypes';
import type { OpenDeleteAliasDestinationBlockedModal } from '../../../../shared/hooks/modalControls/types';
import './style.scss';

const modalNameValue = ModalName.DeleteAliasDestinationBlocked;

type ModalData = OpenDeleteAliasDestinationBlockedModal;

export const DeletionBlockedModal = () => {
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
      id="deletion-blocked-modal"
      title={modalData?.title ?? ''}
      size="small"
      isOpen={isOpen}
      onClose={() => setOpen(false)}
      afterClose={() => setModalData(null)}
    >
      <div className="content">
        <AppText className="description" font={TextStyle.TBodySm400}>
          {modalData?.description ?? ''}
        </AppText>
        <ul className="rules-list">
          {(modalData?.rules ?? []).map((rule, index) => (
            <li key={`${rule}-${index}`}>{rule}</li>
          ))}
        </ul>
        <Controls>
          <div className="right">
            <Button
              variant="secondary"
              text={m.controls_close()}
              onClick={() => setOpen(false)}
            />
          </div>
        </Controls>
      </div>
    </Modal>
  );
};
