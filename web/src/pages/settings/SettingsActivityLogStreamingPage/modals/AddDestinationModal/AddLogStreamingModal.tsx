import { ModalName } from '../../../../../shared/hooks/modalControls/modalTypes';
import { useEffect, useState } from 'react';
import {
  subscribeCloseModal,
  subscribeOpenModal,
} from '../../../../../shared/hooks/modalControls/modalsSubjects';
import { Modal } from '../../../../../shared/defguard-ui/components/Modal/Modal';
import { SectionSelect } from '../../../../../shared/defguard-ui/components/SectionSelect/SectionSelect';
import { useAddDestinationModal } from './useAddDestinationModal';
import { SizedBox } from '../../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../../../shared/defguard-ui/types';
import { m } from '../../../../../paraglide/messages';

const modalNameValue = ModalName.AddLogStreaming;

export const AddLogStreamingModal = () => {
  const [isOpen, setOpen] = useState(false);

  useEffect(() => {
    const openSub = subscribeOpenModal(modalNameValue, () => {
      setOpen(true);
    });
    const closeSub = subscribeCloseModal(modalNameValue, () => setOpen(false));
    return () => {
      openSub.unsubscribe();
      closeSub.unsubscribe();
    };
  }, []);

  return (
    <Modal title="Select destination" isOpen={isOpen} onClose={() => setOpen(false)}>
      <ModalContent />
    </Modal>
  );
};

const ModalContent = () => {
  return (
    <>
      <SectionSelect
        image="logstash"
        title="Logstash"
        content={m.modal_add_logstash_destination()}
        data-testid="add-logstash"
        onClick={() => {
          // useAddUserModal.setState({
          //   step: 'user',
          //   enrollUser: false,
          // });
          useAddDestinationModal.setState({
            destination: 'logstash',
          });
        }}
      />
      <SizedBox height={ThemeSpacing.Md} />
      <SectionSelect
        image="vector"
        content={m.modal_add_vector_destination()}
        title="Vector"
        data-testid="add-vector"
        onClick={() => {
          //todo
        }}
      />
    </>
  );
};
