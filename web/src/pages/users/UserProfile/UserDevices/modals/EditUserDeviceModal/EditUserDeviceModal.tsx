import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { ModalWithTitle } from '../../../../../../shared/components/layout/ModalWithTitle/ModalWithTitle';
import { useModalStore } from '../../../../../../shared/hooks/store/useModalStore';
import { EditUserDeviceForm } from './UserDeviceEditForm';
export const EditUserDeviceModal = () => {
  const { LL } = useI18nContext();
  const modalState = useModalStore((state) => state.editUserDeviceModal);
  const setModalsState = useModalStore((state) => state.setState);
  return (
    <ModalWithTitle
      id="edit-user-device"
      title={LL.modals.editDevice.title()}
      isOpen={modalState.visible}
      setIsOpen={(v) => setModalsState({ editUserDeviceModal: { visible: v } })}
      backdrop
    >
      <EditUserDeviceForm />
    </ModalWithTitle>
  );
};
