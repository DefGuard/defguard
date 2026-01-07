import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { ModalWithTitle } from '../../../../../../shared/defguard-ui/components/Layout/modals/ModalWithTitle/ModalWithTitle';
import { ChangeSelfPasswordForm } from './components/ChangeSelfPasswordForm';
import { useChangeSelfPasswordModal } from './hooks/useChangeSelfPasswordModal';

export const ChangeSelfPasswordModal = () => {
  const { LL } = useI18nContext();
  const visible = useChangeSelfPasswordModal((state) => state.visible);
  const resetModal = useChangeSelfPasswordModal((state) => state.reset);
  return (
    <ModalWithTitle
      isOpen={visible}
      setIsOpen={() => resetModal()}
      title={LL.modals.changePasswordSelf.title()}
      backdrop
    >
      <ChangeSelfPasswordForm />
    </ModalWithTitle>
  );
};
