import parse from 'html-react-parser';
import { isUndefined } from 'lodash-es';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { Button } from '../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../shared/defguard-ui/components/Layout/Button/types';
import { Input } from '../../../../shared/defguard-ui/components/Layout/Input/Input';
import { MessageBox } from '../../../../shared/defguard-ui/components/Layout/MessageBox/MessageBox';
import { MessageBoxType } from '../../../../shared/defguard-ui/components/Layout/MessageBox/types';
import {
  SelectOption,
} from '../../../../shared/defguard-ui/components/Layout/Select/types';
import useApi from '../../../../shared/hooks/useApi';
import { useClipboard } from '../../../../shared/hooks/useClipboard';
import { useToaster } from '../../../../shared/hooks/useToaster';
import {
  AddDeviceConfig,
  DeviceNetworkInfo,
  GetDeviceConfigRequest,
} from '../../../../shared/types';
import { downloadWGConfig } from '../../../../shared/utils/downloadWGConfig';
import {
  DeviceModalSetupMode,
  useDeviceModal,
} from '../../../users/UserProfile/UserDevices/hooks/useDeviceModal';
import { Card } from '../../../../shared/defguard-ui/components/Layout/Card/Card';

export const ConfigStep = () => {
  const { writeToClipboard } = useClipboard();
  const [configsData, deviceName, setupMode, device] = useDeviceModal(
    (state) => [state.configs, state.deviceName, state.setupMode, state.device],
    shallow,
  );
  const nextStep = useDeviceModal((state) => state.nextStep);
  const { LL } = useI18nContext();
  const [selectedNetwork, setSelectedNetwork] = useState<number | undefined>();
  const [selectedConfig, setSelectedConfig] = useState<string | undefined>();
  // download networks and configs form API instead
  const standAloneMode = isUndefined(configsData);





  const getWarningMessageConent = useMemo(() => {
    if (setupMode === DeviceModalSetupMode.AUTO_CONFIG) {
      return parse(LL.modals.addDevice.web.steps.config.helpers.warningAutoMode());
    }
    return parse(LL.modals.addDevice.web.steps.config.helpers.warningManualMode());
  }, [LL.modals.addDevice.web.steps.config.helpers, setupMode]);


  // init select on mount
  useEffect(() => {
    if (!standAloneMode && configsData && configsData.length) {
      setSelectedNetwork(configsData[0].network_id);
    }

    if (standAloneMode && device && device.networks.length) {
      setSelectedNetwork(device.networks[0].network_id);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // set correct config after selected, only setup mode, standalone uses query
  useEffect(() => {
    if (!standAloneMode && configsData && selectedNetwork) {
      const deviceData = configsData.find((c) => c.network_id === selectedNetwork);
      if (deviceData) {
        setSelectedConfig(deviceData.config);
      }
    }
  }, [configsData, selectedNetwork, standAloneMode]);

  return (
    <Card id="add-device-config-step" shaded>
      <MessageBox type={MessageBoxType.WARNING}>{getWarningMessageConent}</MessageBox>
      <Input
        label={LL.modals.addDevice.web.steps.config.inputNameLabel()}
        value={deviceName || device?.name || ''}
        onChange={() => {
          return;
        }}
        disabled={true}
      />
      <div className="info">
        <p>{LL.modals.addDevice.web.steps.config.qrInfo()}</p>
      </div>
      <div className="controls">
        <Button
          text={LL.form.close()}
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.STANDARD}
          onClick={() => nextStep()}
        />
      </div>
    </Card>
  );
};



