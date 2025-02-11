import { useEffect } from 'react';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { WireguardConfigExpandable } from '../../../../shared/components/Layout/WireguardConfigExpandable/WireguardConfigExpandable';
import { Button } from '../../../../shared/defguard-ui/components/Layout/Button/Button';
import { ModalWithTitle } from '../../../../shared/defguard-ui/components/Layout/modals/ModalWithTitle/ModalWithTitle';
import { useStandaloneDeviceConfigModal } from './store';

export const StandaloneDeviceConfigModal = () => {
  const { LL } = useI18nContext();
  const isOpen = useStandaloneDeviceConfigModal((s) => s.visible);
  const [close, reset] = useStandaloneDeviceConfigModal(
    (s) => [s.close, s.reset],
    shallow,
  );

  useEffect(() => {
    return () => {
      reset();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <ModalWithTitle
      title={LL.modals.standaloneDeviceConfigModal.title()}
      isOpen={isOpen}
      onClose={close}
      afterClose={reset}
      id="standalone-device-config-modal"
      includeDefaultStyles
    >
      <ModalContent />
    </ModalWithTitle>
  );
};

const ModalContent = () => {
  const { LL } = useI18nContext();
  const data = useStandaloneDeviceConfigModal((s) => s.data);
  const close = useStandaloneDeviceConfigModal((s) => s.close, shallow);

  if (!data) return null;
  return (
    <>
      <WireguardConfigExpandable
        config={data.config}
        deviceName={data.device.name}
        // modal can't be opened when configured is false means pubkey will be always a string here
        publicKey={data.device.wireguard_pubkey as string}
      />
      <div className="controls solo">
        <Button
          className="cancel"
          onClick={() => close()}
          text={LL.common.controls.close()}
        />
      </div>
    </>
  );
};
