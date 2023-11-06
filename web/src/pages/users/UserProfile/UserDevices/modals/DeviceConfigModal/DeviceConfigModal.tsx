import './style.scss';

import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { DeviceConfigsCard } from '../../../../../../shared/components/network/DeviceConfigsCard/DeviceConfigsCard';
import { ModalWithTitle } from '../../../../../../shared/defguard-ui/components/Layout/modals/ModalWithTitle/ModalWithTitle';
import { useDeviceConfigModal } from '../../hooks/useDeviceConfigModal';

export const DeviceConfigModal = () => {
  const isOpen = useDeviceConfigModal((state) => state.isOpen);
  const [close, reset] = useDeviceConfigModal(
    (state) => [state.close, state.reset],
    shallow,
  );
  const { LL } = useI18nContext();

  return (
    <ModalWithTitle
      id="device-config-modal"
      isOpen={isOpen}
      title={LL.modals.deviceConfig.title()}
      onClose={() => close()}
      afterClose={() => reset()}
    >
      <ModalContent />
    </ModalWithTitle>
  );
};

const ModalContent = () => {
  const [networks, userId, deviceId, publicKey, deviceName] = useDeviceConfigModal(
    (state) => [
      state.networks,
      state.userId,
      state.deviceId,
      state.publicKey,
      state.deviceName,
    ],
    shallow,
  );

  if (!networks || !userId || !deviceId || !publicKey || !deviceName) return null;

  return (
    <DeviceConfigsCard
      deviceId={deviceId}
      publicKey={publicKey}
      userId={userId}
      networks={networks}
      deviceName={deviceName}
      privateKey={undefined}
    />
  );
};
