import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { ModalWithTitle } from '../../../../../../shared/components/layout/ModalWithTitle/ModalWithTitle';
import { useEditDeviceModal } from '../../hooks/useEditDeviceModal';
import { EditUserDeviceForm } from './UserDeviceEditForm';

export const EditUserDeviceModal = () => {
  const { LL } = useI18nContext();
  const visible = useEditDeviceModal((state) => state.visible);
  const closeModal = useEditDeviceModal((state) => state.close);

  return (
    <ModalWithTitle
      id="edit-user-device"
      title={LL.modals.editDevice.title()}
      isOpen={visible}
      setIsOpen={() => closeModal()}
      backdrop
    >
      <EditUserDeviceForm />
    </ModalWithTitle>
  );
};
