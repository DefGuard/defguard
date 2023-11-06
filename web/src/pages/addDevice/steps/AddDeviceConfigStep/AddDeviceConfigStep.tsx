import './style.scss';

import parse from 'html-react-parser';
import { isUndefined } from 'lodash-es';
import { useEffect, useMemo } from 'react';
import { useNavigate } from 'react-router';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { DeviceConfigsCard } from '../../../../shared/components/network/DeviceConfigsCard/DeviceConfigsCard';
import { Card } from '../../../../shared/defguard-ui/components/Layout/Card/Card';
import { Input } from '../../../../shared/defguard-ui/components/Layout/Input/Input';
import { MessageBox } from '../../../../shared/defguard-ui/components/Layout/MessageBox/MessageBox';
import { MessageBoxType } from '../../../../shared/defguard-ui/components/Layout/MessageBox/types';
import { useAddDevicePageStore } from '../../hooks/useAddDevicePageStore';

enum SetupMode {
  AUTO,
  MANUAL,
}

export const AddDeviceConfigStep = () => {
  const { LL } = useI18nContext();
  const localLL = LL.addDevicePage.steps.configDevice;
  const navigate = useNavigate();

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

  const nextSubject = useAddDevicePageStore((state) => state.nextSubject, shallow);
  const resetPageState = useAddDevicePageStore((state) => state.reset);

  const setupMode = isUndefined(privateKey) ? SetupMode.MANUAL : SetupMode.AUTO;

  const getWarningMessageConent = useMemo(() => {
    if (setupMode === SetupMode.AUTO) {
      return parse(localLL.helpers.warningAutoMode());
    }
    return parse(localLL.helpers.warningManualMode());
  }, [localLL.helpers, setupMode]);

  useEffect(() => {
    const sub = nextSubject.subscribe(() => {
      navigate(-1);
      setTimeout(() => {
        resetPageState();
      }, 1000);
    });
    return () => {
      sub.unsubscribe();
    };
  }, [navigate, nextSubject, resetPageState]);

  if (!device || !userData || !publicKey || !networks) return null;

  return (
    <Card id="add-device-config-step" shaded>
      <h2>{localLL.title()}</h2>
      <MessageBox type={MessageBoxType.WARNING}>{getWarningMessageConent}</MessageBox>
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
      <DeviceConfigsCard
        deviceId={device.id}
        publicKey={publicKey}
        privateKey={privateKey}
        userId={userData.id}
        networks={networks}
        deviceName={device.name}
      />
    </Card>
  );
};
