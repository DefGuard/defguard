import { useI18nContext } from '../../../../../i18n/i18n-react';
import { ModalWithTitle } from '../../../../../shared/components/layout/ModalWithTitle/ModalWithTitle';
import { useModalStore } from '../../../../../shared/hooks/store/useModalStore';
import { ChangePasswordForm } from './ChangePasswordForm';

export const ChangePasswordModal = () => {
  const modalState = useModalStore((state) => state.changePasswordModal);
  const setModalState = useModalStore((state) => state.setChangePasswordModal);
  const { LL } = useI18nContext();

  return (
    <ModalWithTitle
      title={LL.modals.changeUserPassword.title()}
      isOpen={modalState.visible}
      setIsOpen={(visibility) => setModalState({ visible: visibility })}
      backdrop
    >
      <ChangePasswordForm />
    </ModalWithTitle>
  );
};
