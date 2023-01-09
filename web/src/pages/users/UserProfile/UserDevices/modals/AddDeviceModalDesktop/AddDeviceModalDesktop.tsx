import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { ModalWithTitle } from '../../../../../../shared/components/layout/ModalWithTitle/ModalWithTitle';
import { useModalStore } from '../../../../../../shared/hooks/store/useModalStore';
import { AddDeviceDesktopForm } from './AddDeviceDesktopForm';

export const AddDeviceModalDesktop = () => {
  const visible = useModalStore((state) => state.addDeviceDesktopModal.visible);
  const setModalsState = useModalStore((state) => state.setState);
  const { LL } = useI18nContext();
  return (
    <ModalWithTitle
      title={LL.modals.addDevice.desktop.title()}
      isOpen={visible}
      setIsOpen={(v) =>
        setModalsState({ addDeviceDesktopModal: { visible: v } })
      }
      backdrop
    >
      <AddDeviceDesktopForm />
    </ModalWithTitle>
  );
};
