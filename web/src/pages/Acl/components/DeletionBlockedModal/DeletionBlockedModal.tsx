import { Modal } from '../../../../shared/defguard-ui/components/Modal/Modal';
import { ModalControls } from '../../../../shared/defguard-ui/components/ModalControls/ModalControls';
import './style.scss';

type Props = {
  isOpen: boolean;
  title: string;
  description: string;
  rules: string[];
  onClose: () => void;
};

export const DeletionBlockedModal = ({
  isOpen,
  title,
  description,
  rules,
  onClose,
}: Props) => (
  <Modal
    id="deletion-blocked-modal"
    title={title}
    size="small"
    isOpen={isOpen}
    onClose={onClose}
  >
    <div className="deletion-blocked-modal-content">
      <p className="deletion-blocked-modal-description">{description}</p>
      <ul className="deletion-blocked-modal-list">
        {rules.map((rule) => (
          <li key={rule}>{rule}</li>
        ))}
      </ul>
      <ModalControls
        submitProps={{
          text: 'Close',
          onClick: onClose,
        }}
      />
    </div>
  </Modal>
);
