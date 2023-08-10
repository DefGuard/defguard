import { useQuery } from '@tanstack/react-query';
import parse from 'html-react-parser';
import { isUndefined } from 'lodash-es';
import { useCallback, useEffect, useMemo, useState } from 'react';
import QRCode from 'react-qr-code';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../../../../i18n/i18n-react';
import { ActionButton } from '../../../../../../../shared/defguard-ui/components/Layout/ActionButton/ActionButton';
import { ActionButtonVariant } from '../../../../../../../shared/defguard-ui/components/Layout/ActionButton/types';
import { Button } from '../../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../../shared/defguard-ui/components/Layout/Button/types';
import { ExpandableCard } from '../../../../../../../shared/defguard-ui/components/Layout/ExpandableCard/ExpandableCard';
import { Input } from '../../../../../../../shared/defguard-ui/components/Layout/Input/Input';
import { LoaderSpinner } from '../../../../../../../shared/defguard-ui/components/Layout/LoaderSpinner/LoaderSpinner';
import { MessageBox } from '../../../../../../../shared/defguard-ui/components/Layout/MessageBox/MessageBox';
import { MessageBoxType } from '../../../../../../../shared/defguard-ui/components/Layout/MessageBox/types';
import { Select } from '../../../../../../../shared/defguard-ui/components/Layout/Select/Select';
import {
  SelectOption,
  SelectSizeVariant,
} from '../../../../../../../shared/defguard-ui/components/Layout/Select/types';
import useApi from '../../../../../../../shared/hooks/useApi';
import { useClipboard } from '../../../../../../../shared/hooks/useClipboard';
import { useToaster } from '../../../../../../../shared/hooks/useToaster';
import { QueryKeys } from '../../../../../../../shared/queries';
import {
  AddDeviceConfig,
  DeviceNetworkInfo,
  GetDeviceConfigRequest,
} from '../../../../../../../shared/types';
import { downloadWGConfig } from '../../../../../../../shared/utils/downloadWGConfig';
import { DeviceModalSetupMode, useDeviceModal } from '../../../hooks/useDeviceModal';

export const ConfigStep = () => {
  const { writeToClipboard } = useClipboard();
  const {
    device: { downloadDeviceConfig },
  } = useApi();
  const [configsData, deviceName, setupMode, device] = useDeviceModal(
    (state) => [state.configs, state.deviceName, state.setupMode, state.device],
    shallow,
  );
  const nextStep = useDeviceModal((state) => state.nextStep);
  const { LL, locale } = useI18nContext();
  const toaster = useToaster();
  const [selectedNetwork, setSelectedNetwork] = useState<number | undefined>();
  const [selectedConfig, setSelectedConfig] = useState<string | undefined>();
  // download networks and configs form API instead
  const standAloneMode = isUndefined(configsData);

  const queryParams = useMemo((): GetDeviceConfigRequest | undefined => {
    if (device && selectedNetwork && standAloneMode) {
      return {
        network_id: selectedNetwork,
        device_id: device.id,
      };
    }
    return undefined;
  }, [device, standAloneMode, selectedNetwork]);

  const { isLoading: loadingConfig } = useQuery(
    [QueryKeys.FETCH_DEVICE_CONFIG, queryParams],
    () => downloadDeviceConfig(queryParams as GetDeviceConfigRequest),
    {
      enabled: !!queryParams && standAloneMode,
      onSuccess: (res) => {
        setSelectedConfig(
          res.replace('YOUR_PRIVATE_KEY', device?.wireguard_pubkey ?? ''),
        );
      },
    },
  );

  const handleConfigDownload = useCallback(() => {
    if (standAloneMode) {
      if (!loadingConfig && selectedConfig && device) {
        const network = device.networks.find((n) => n.network_id === selectedNetwork);
        if (network) {
          downloadWGConfig(
            selectedConfig,
            `${device?.name.toLowerCase().replace(' ', '')}-${network?.network_name
              .toLowerCase()
              .replace(' ', '')}`,
          );
        }
      }
    } else {
      if (selectedConfig) {
        const data = configsData.find((c) => c.network_id === selectedNetwork);
        if (data) {
          downloadWGConfig(
            selectedConfig,
            `${deviceName?.toLowerCase().replace(' ', '')}-${data.network_name
              .toLowerCase()
              .replace(' ', '')}`,
          );
        }
      }
    }
  }, [
    configsData,
    device,
    deviceName,
    loadingConfig,
    selectedConfig,
    standAloneMode,
    selectedNetwork,
  ]);

  const expandableCardActions = useMemo(() => {
    return [
      <ActionButton variant={ActionButtonVariant.QRCODE} key={1} active={true} />,
      <ActionButton
        variant={ActionButtonVariant.COPY}
        key={2}
        onClick={() => {
          if (selectedConfig) {
            writeToClipboard(
              selectedConfig,
              LL.modals.addDevice.web.steps.config.messages.copyConfig(),
            );
          }
        }}
        disabled={isUndefined(selectedConfig)}
      />,
      <ActionButton
        variant={ActionButtonVariant.DOWNLOAD}
        key={3}
        onClick={() => handleConfigDownload()}
        disabled={isUndefined(selectedConfig)}
      />,
    ];
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [configsData, deviceName, toaster, locale]);

  const getSelectOptions = useMemo(() => {
    if (!standAloneMode && configsData) {
      return configsData.map((c) => configToSelectOption(c));
    }
    if (standAloneMode && device) {
      return device.networks.map((n) => networkInfoToSelectOption(n));
    }
    return [];
  }, [configsData, device, standAloneMode]);

  const getWarningMessageConent = useMemo(() => {
    if (setupMode === DeviceModalSetupMode.AUTO_CONFIG) {
      return parse(LL.modals.addDevice.web.steps.config.helpers.warningAutoMode());
    }
    return parse(LL.modals.addDevice.web.steps.config.helpers.warningManualMode());
  }, [LL.modals.addDevice.web.steps.config.helpers, setupMode]);

  const getExpandCardExtras = useMemo(() => {
    return (
      <Select
        selected={selectedNetwork}
        options={getSelectOptions}
        searchable={false}
        sizeVariant={SelectSizeVariant.STANDARD}
        loading={standAloneMode && loadingConfig}
        onChangeSingle={(networkId) => {
          setSelectedNetwork(networkId);
        }}
      />
    );
  }, [getSelectOptions, loadingConfig, standAloneMode, selectedNetwork]);

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
    <>
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
      <ExpandableCard
        title={LL.modals.addDevice.web.steps.config.qrCardTitle()}
        actions={expandableCardActions}
        topExtras={getExpandCardExtras}
        expanded
      >
        {selectedConfig && <QRCode value={selectedConfig} size={250} />}
        {isUndefined(selectedConfig) && <LoaderSpinner size={250} />}
      </ExpandableCard>
      <div className="controls">
        <Button
          text={LL.form.close()}
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.STANDARD}
          onClick={() => nextStep()}
        />
      </div>
    </>
  );
};

const networkInfoToSelectOption = (info: DeviceNetworkInfo): SelectOption<number> => ({
  value: info.network_id,
  label: info.network_name,
  key: info.network_id,
});

const configToSelectOption = (configData: AddDeviceConfig): SelectOption<number> => ({
  value: configData.network_id,
  label: configData.network_name,
  key: configData.network_id,
});
