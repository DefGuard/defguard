import { m } from '../../../../paraglide/messages';
import { FormSection } from '../../../../shared/components/FormSection/FormSection';
import { Button } from '../../../../shared/defguard-ui/components/Button/Button';
import { CodeBox } from '../../../../shared/defguard-ui/components/CodeBox/CodeBox';
import { InfoBanner } from '../../../../shared/defguard-ui/components/InfoBanner/InfoBanner';
import { Modal } from '../../../../shared/defguard-ui/components/Modal/Modal';
import { ModalControls } from '../../../../shared/defguard-ui/components/ModalControls/ModalControls';
import { SizedBox } from '../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { useClipboard } from '../../../../shared/defguard-ui/hooks/useClipboard';
import { ThemeSpacing } from '../../../../shared/defguard-ui/types';
import { isPresent } from '../../../../shared/defguard-ui/utils/isPresent';
import {
  closeModal,
  subscribeCloseModal,
  subscribeOpenModal,
} from '../../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../../shared/hooks/modalControls/modalTypes';
import type { OpenNetworkDeviceConfigModal } from '../../../../shared/hooks/modalControls/types';
import { downloadText } from '../../../../shared/utils/download';
import { formatFileName } from '../../../../shared/utils/formatFileName';
import './style.scss';
import { useEffect, useState } from 'react';

const modalNameValue = ModalName.NetworkDeviceConfig;

type ModalData = OpenNetworkDeviceConfigModal;

export const NetworkDeviceConfigModal = () => {
  const [isOpen, setOpen] = useState(false);
  const [modalData, setModalData] = useState<ModalData | null>(null);

  useEffect(() => {
    const openSub = subscribeOpenModal(modalNameValue, (data) => {
      setModalData(data);
      setOpen(true);
    });
    const closeSub = subscribeCloseModal(modalNameValue, () => setOpen(false));
    return () => {
      openSub.unsubscribe();
      closeSub.unsubscribe();
    };
  }, []);

  return (
    <Modal
      id="network-device-config-modal"
      title={'Device VPN configuration'}
      isOpen={isOpen}
      onClose={() => setOpen(false)}
      afterClose={() => {
        setModalData(null);
      }}
    >
      {isPresent(modalData) && <ModalContent {...modalData} />}
    </Modal>
  );
};

const ModalContent = ({ config, device }: ModalData) => {
  const { writeToClipboard } = useClipboard();
  return (
    <>
      <FormSection
        title="Get configuration file"
        text="Use the provided configuration file by importing it into your device's WireGuard app."
      />
      <SizedBox height={ThemeSpacing.Xl2} />
      <InfoBanner
        variant="warning"
        icon="warning"
        text={
          "Defguard doesn't store private keys. Keys are generated in your browser — only the public key is saved. Download the configuration now; the private key won't be available later."
        }
      />
      <SizedBox height={ThemeSpacing.Xl2} />
      <CodeBox text={config.replaceAll('\n', '<br/>')} markdown />
      <SizedBox height={ThemeSpacing.Xl2} />
      <div className="box-controls">
        <Button
          variant="outlined"
          text="Download config file"
          iconLeft="download"
          onClick={() => {
            downloadText(config, formatFileName(device.name), 'conf');
          }}
        />
        <Button
          variant="outlined"
          text={m.controls_copy_clipboard()}
          iconLeft="copy"
          onClick={() => {
            writeToClipboard(config);
          }}
        />
      </div>
      <ModalControls
        submitProps={{
          text: m.controls_close(),
          onClick: () => {
            closeModal(modalNameValue);
          },
        }}
      />
    </>
  );
};
