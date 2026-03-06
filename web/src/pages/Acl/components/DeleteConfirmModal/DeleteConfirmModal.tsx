import { AppText } from '../../../../shared/defguard-ui/components/AppText/AppText';
import { Modal } from '../../../../shared/defguard-ui/components/Modal/Modal';
import { ModalControls } from '../../../../shared/defguard-ui/components/ModalControls/ModalControls';
import { TextStyle } from '../../../../shared/defguard-ui/types';

type Props = {
  isOpen: boolean;
  title: string;
  description: string;
  onConfirm: () => void;
  onClose: () => void;
  isPending?: boolean;
};

export const DeleteConfirmModal = ({
  isOpen,
  title,
  description,
  onConfirm,
  onClose,
  isPending,
}: Props) => (
  <Modal
    id="delete-confirm-modal"
    title={title}
    size="small"
    isOpen={isOpen}
    onClose={onClose}
  >
    <AppText font={TextStyle.TBodySm400}>{description}</AppText>
    <ModalControls
      submitProps={{
        text: 'Delete',
        variant: 'critical',
        onClick: onConfirm,
        loading: isPending,
        disabled: isPending,
      }}
      cancelProps={{
        text: 'Cancel',
        onClick: onClose,
        disabled: isPending,
      }}
    />
  </Modal>
);
