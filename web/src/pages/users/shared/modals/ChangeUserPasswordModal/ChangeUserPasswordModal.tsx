import { ModalWithTitle } from '../../../../../shared/components/layout/ModalWithTitle/ModalWithTitle';
import { useModalStore } from '../../../../../shared/hooks/store/useModalStore';
import { ChangePasswordForm } from './ChangePasswordForm';

export const ChangePasswordModal = () => {
  const modalState = useModalStore((state) => state.changePasswordModal);
  const setModalState = useModalStore((state) => state.setChangePasswordModal);

  return (
    <ModalWithTitle
      title="Change user password"
      isOpen={modalState.visible}
      setIsOpen={(visibility) => setModalState({ visible: visibility })}
      backdrop
    >
      <ChangePasswordForm />
    </ModalWithTitle>
  );
};
