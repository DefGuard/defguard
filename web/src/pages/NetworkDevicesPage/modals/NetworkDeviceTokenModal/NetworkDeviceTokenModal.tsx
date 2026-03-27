import { useEffect, useState } from 'react';
import { m } from '../../../../paraglide/messages';
import { AppText } from '../../../../shared/defguard-ui/components/AppText/AppText';
import { CopyField } from '../../../../shared/defguard-ui/components/CopyField/CopyField';
import { Modal } from '../../../../shared/defguard-ui/components/Modal/Modal';
import { ModalControls } from '../../../../shared/defguard-ui/components/ModalControls/ModalControls';
import { SizedBox } from '../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import {
  TextStyle,
  ThemeSpacing,
  ThemeVariable,
} from '../../../../shared/defguard-ui/types';
import { isPresent } from '../../../../shared/defguard-ui/utils/isPresent';
import {
  closeModal,
  subscribeCloseModal,
  subscribeOpenModal,
} from '../../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../../shared/hooks/modalControls/modalTypes';
import type { OpenNetworkDeviceTokenModal } from '../../../../shared/hooks/modalControls/types';

const modalNameValue = ModalName.NetworkDeviceToken;

type ModalData = OpenNetworkDeviceTokenModal;

export const NetworkDeviceTokenModal = () => {
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
      id="network-device-token-modal"
      title={m.modal_network_device_token_title()}
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

const ModalContent = ({ enrollment }: ModalData) => {
  const command = `dg enroll -u ${enrollment.enrollment_url} -t ${enrollment.enrollment_token}`;
  return (
    <>
      <AppText font={TextStyle.TBodySm500}>
        {m.modal_network_device_cli_step_title()}
      </AppText>
      <SizedBox height={ThemeSpacing.Xs} />
      <AppText font={TextStyle.TBodySm400} color={ThemeVariable.FgMuted}>
        {m.modal_network_device_cli_step_content()}
      </AppText>
      <SizedBox height={ThemeSpacing.Xl2} />
      <CopyField
        data-testid="copy-field"
        label={m.form_label_command()}
        text={command}
        copyTooltip={m.misc_clipboard_copy()}
      />
      <ModalControls
        submitProps={{
          text: m.controls_close(),
          testId: 'close',
          onClick: () => {
            closeModal(modalNameValue);
          },
        }}
      />
    </>
  );
};
