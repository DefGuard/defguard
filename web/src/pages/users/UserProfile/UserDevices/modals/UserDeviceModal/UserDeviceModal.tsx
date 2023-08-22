import './style.scss';

import { isUndefined } from 'lodash-es';
import { ReactNode } from 'react';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { ModalWithTitle } from '../../../../../../shared/defguard-ui/components/Layout/modals/ModalWithTitle/ModalWithTitle';
import { DeviceModalSetupMode, useDeviceModal } from '../../hooks/useDeviceModal';
import { ConfigStep } from './steps/ConfigStep';
import { SetupStep } from './steps/SetupStep';

const modalSteps: ReactNode[] = [<SetupStep key={0} />, <ConfigStep key={1} />];

export const UserDeviceModal = () => {
  const [visible, currentStep, device, mode] = useDeviceModal(
    (state) => [state.visible, state.currentStep, state.device, state.setupMode],
    shallow,
  );
  const { LL } = useI18nContext();
  const setDeviceModal = useDeviceModal((state) => state.setState);

  const title = () => {
    if (
      currentStep === 1 &&
      !isUndefined(device) &&
      mode === DeviceModalSetupMode.MANUAL_CONFIG
    ) {
      return LL.modals.addDevice.web.viewTitle();
    }
    return LL.modals.addDevice.web.title();
  };

  return (
    <ModalWithTitle
      id="add-user-device-modal"
      title={title()}
      isOpen={visible}
      setIsOpen={() => setDeviceModal({ visible: false })}
      steps={modalSteps}
      currentStep={currentStep}
      backdrop
    />
  );
};
