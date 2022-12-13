import { ModalWithTitle } from '../../../../../../shared/components/layout/ModalWithTitle/ModalWithTitle';
import { useModalStore } from '../../../../../../shared/hooks/store/useModalStore';
import { AddDeviceDesktopForm } from './AddDeviceDesktopForm';

export const AddDeviceModalDesktop = () => {
  const visible = useModalStore((state) => state.addDeviceDesktopModal.visible);
  const setModalsState = useModalStore((state) => state.setState);
  return (
    <ModalWithTitle
      title="Add current device"
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
