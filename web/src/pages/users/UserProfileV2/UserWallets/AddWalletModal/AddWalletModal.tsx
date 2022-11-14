import './style.scss';

import MessageBox, {
  MessageBoxType,
} from '../../../../../shared/components/layout/MessageBox/MessageBox';
import { ModalWithTitle } from '../../../../../shared/components/layout/ModalWithTitle/ModalWithTitle';
import { useModalStore } from '../../../../../shared/hooks/store/useModalStore';
import { AddWalletModalForm } from './AddWalletModalForm';

export const AddWalletModal = () => {
  const open = useModalStore((state) => state.addWalletModal.visible);
  const setModalsState = useModalStore((state) => state.setState);

  return (
    <ModalWithTitle
      id="add-wallet-modal"
      title="Add wallet"
      isOpen={open}
      setIsOpen={(visibility) =>
        setModalsState({ addWalletModal: { visible: visibility } })
      }
      backdrop
    >
      <MessageBox type={MessageBoxType.INFO}>
        <p>In order to add a ETH wallet you will need to sign message.</p>
      </MessageBox>
      <AddWalletModalForm />
    </ModalWithTitle>
  );
};
