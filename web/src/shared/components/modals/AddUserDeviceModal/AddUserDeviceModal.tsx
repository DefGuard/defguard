import { useEffect, useMemo } from 'react';
import { useShallow } from 'zustand/react/shallow';
import { m } from '../../../../paraglide/messages';
import { Modal } from '../../../defguard-ui/components/Modal/Modal';
import { AddDeviceModalClientConfigStep } from './steps/AddDeviceModalClientConfigStep/AddDeviceModalClientConfigStep';
import { AddDeviceModalManualDownloadStep } from './steps/AddDeviceModalManualDownloadStep/AddDeviceModalManualDownloadStep';
import { AddDeviceModalManualSetupStep } from './steps/AddDeviceModalManualSetupStep/AddDeviceModalManualSetupStep';
import { AddDeviceModalStartStep } from './steps/AddDeviceModalStartStep/AddDeviceModalStartStep';
import { useAddUserDeviceModal } from './store/useAddUserDeviceModal';

export const AddUserDeviceModal = () => {
  const isOpen = useAddUserDeviceModal((s) => s.isOpen);
  const [closeModal, resetModal] = useAddUserDeviceModal(
    useShallow((s) => [s.close, s.reset]),
  );

  const activeStep = useAddUserDeviceModal((s) => s.step);

  const modalTitle = useMemo(() => {
    switch (activeStep) {
      case 'start-choice':
        return m.modal_add_user_device_title_add();
      case 'client-setup':
        return m.modal_add_user_device_title_add();
      case 'manual-setup':
        return m.modal_add_user_device_title_manual();
      case 'manual-configuration':
        return m.modal_add_user_device_title_save();
    }
  }, [activeStep]);

  // biome-ignore lint/correctness/useExhaustiveDependencies: onUnmount
  useEffect(() => {
    return () => {
      resetModal();
    };
  }, []);

  return (
    <Modal
      id="add-user-device-modal"
      size="primary"
      title={modalTitle}
      isOpen={isOpen}
      onClose={() => {
        closeModal();
      }}
      afterClose={() => {
        resetModal();
      }}
    >
      <ModalContent />
    </Modal>
  );
};

const ModalContent = () => {
  const currentStep = useAddUserDeviceModal((s) => s.step);

  const RenderStep = useMemo(() => {
    switch (currentStep) {
      case 'start-choice':
        return AddDeviceModalStartStep;
      case 'client-setup':
        return AddDeviceModalClientConfigStep;
      case 'manual-configuration':
        return AddDeviceModalManualDownloadStep;
      case 'manual-setup':
        return AddDeviceModalManualSetupStep;
    }
  }, [currentStep]);

  return <RenderStep />;
};
