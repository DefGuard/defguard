import './style.scss';

import parse from 'html-react-parser';
import { isUndefined } from 'lodash-es';
import { useEffect, useMemo } from 'react';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { DeviceConfigsCard } from '../../../../shared/components/network/DeviceConfigsCard/DeviceConfigsCard';
import { Card } from '../../../../shared/defguard-ui/components/Layout/Card/Card';
import { Input } from '../../../../shared/defguard-ui/components/Layout/Input/Input';
import { MessageBox } from '../../../../shared/defguard-ui/components/Layout/MessageBox/MessageBox';
import { MessageBoxType } from '../../../../shared/defguard-ui/components/Layout/MessageBox/types';
import { useAddDevicePageStore } from '../../hooks/useAddDevicePageStore';
import { AddDeviceStep } from '../../types';

enum SetupMode {
  AUTO,
  MANUAL,
}

export const AddDeviceConfigStep = () => {
  const { LL } = useI18nContext();
  const localLL = LL.addDevicePage.steps.configDevice;

  const [userData, device, publicKey, privateKey, networks] = useAddDevicePageStore(
    (state) => [
      state.userData,
      state.device,
      state.publicKey,
      state.privateKey,
      state.networks,
    ],
    shallow,
  );

  const setStep = useAddDevicePageStore((state) => state.setStep, shallow);

  const setupMode = isUndefined(privateKey) ? SetupMode.MANUAL : SetupMode.AUTO;

  const getWarningMessageContent = useMemo(() => {
    if (setupMode === SetupMode.AUTO) {
      return parse(localLL.helpers.warningAutoMode());
    }
    return parse(localLL.helpers.warningManualMode());
  }, [localLL.helpers, setupMode]);

  useEffect(() => {
    if (!device || !userData || !publicKey || !networks) {
      setStep(AddDeviceStep.NATIVE_CHOOSE_METHOD);
    }
  }, [device, networks, publicKey, setStep, userData]);

  if (!device || !userData || !publicKey || !networks) return null;

  return (
    <Card id="add-device-config-step" shaded>
      <h2>{localLL.title()}</h2>
      <MessageBox type={MessageBoxType.WARNING}>{getWarningMessageContent}</MessageBox>
      <Input
        label={localLL.inputNameLabel()}
        value={device.name}
        onChange={() => {
          return;
        }}
        disabled={true}
      />
      <div className="info">
        <p>{localLL.qrInfo()}</p>
      </div>
      {networks.length > 0 && (
        <DeviceConfigsCard
          deviceId={device.id}
          publicKey={publicKey}
          privateKey={privateKey}
          userId={userData.id}
          networks={networks}
          deviceName={device.name}
        />
      )}
      {networks.length === 0 && (
        <MessageBox type={MessageBoxType.WARNING}>
          {localLL.helpers.warningNoNetworks()}
        </MessageBox>
      )}
    </Card>
  );
};
