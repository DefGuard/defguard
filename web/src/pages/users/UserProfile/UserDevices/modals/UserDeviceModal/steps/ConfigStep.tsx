import { useQuery } from '@tanstack/react-query';
import parse from 'html-react-parser';
import { isUndefined } from 'lodash-es';
import { useCallback, useEffect, useMemo, useState } from 'react';
import QRCode from 'react-qr-code';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../../../../i18n/i18n-react';
import useApi from '../../../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../../../shared/hooks/useToaster';
import { QueryKeys } from '../../../../../../../shared/queries';
import {
  AddDeviceConfig,
  DeviceNetworkInfo,
  GetDeviceConfigRequest,
} from '../../../../../../../shared/types';
import { downloadWGConfig } from '../../../../../../../shared/utils/downloadWGConfig';
import { DeviceModalSetupMode, useDeviceModal } from '../../../hooks/useDeviceModal';
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

export const ConfigStep = () => {
  const {
    device: { downloadDeviceConfig },
  } = useApi();
  const [selectedConfig, setSelectedConfig] = useState<string | undefined>();
  const [configsData, deviceName, setupMode, device] = useDeviceModal(
    (state) => [state.configs, state.deviceName, state.setupMode, state.device],
    shallow,
  );
  const nextStep = useDeviceModal((state) => state.nextStep);
  const { LL, locale } = useI18nContext();
  const toaster = useToaster();
  const [selectedConfigOption, setSelectedConfigOption] = useState<
    SelectOption<number> | undefined
  >();

  // download networks and configs form API instead
  const standAloneMode = isUndefined(configsData);

  const queryParams = useMemo((): GetDeviceConfigRequest | undefined => {
    if (device && selectedConfigOption && standAloneMode) {
      return {
        network_id: selectedConfigOption.value,
        device_id: device.id,
      };
    }
    return undefined;
  }, [device, selectedConfigOption, standAloneMode]);

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
      if (
        device &&
        !loadingConfig &&
        selectedConfigOption &&
        selectedConfigOption.value &&
        selectedConfig
      ) {
        const data = device.networks.find(
          (n) => n.network_id === selectedConfigOption.value,
        );
        downloadWGConfig(
          selectedConfig,
          `${device.name.toLowerCase().replace(' ', '')}-${data?.network_name
            .toLowerCase()
            .replace(' ', '')}`,
        );
      }
    } else {
      if (
        selectedConfigOption &&
        selectedConfigOption.value &&
        selectedConfig &&
        configsData
      ) {
        const data = configsData.find(
          (d) => d.network_id === selectedConfigOption?.value,
        );
        downloadWGConfig(
          selectedConfig,
          `${deviceName?.toLowerCase().replace(' ', '')}-${data?.network_name
            .toLowerCase()
            .replace(' ', '')}`,
        );
      }
    }
  }, [
    configsData,
    device,
    deviceName,
    loadingConfig,
    selectedConfig,
    selectedConfigOption,
    standAloneMode,
  ]);

  const expandableCardActions = useMemo(() => {
    return [
      <ActionButton variant={ActionButtonVariant.QRCODE} key={1} forcedActive={true} />,
      <ActionButton
        variant={ActionButtonVariant.COPY}
        key={2}
        onClick={() => {
          if (selectedConfig) {
            clipboard
              .write(selectedConfig)
              .then(() => {
                toaster.success(
                  LL.modals.addDevice.web.steps.config.messages.copyConfig(),
                );
              })
              .catch(() => {
                toaster.error(LL.messages.clipboardError());
              });
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

  const getSelectOptions = useMemo((): SelectOption<number>[] => {
    if (configsData) {
      return configsData.map((c) => configToSelectOption(c));
    }
    if (device) {
      return device.networks.map((n) => networkInfoToSelectOption(n));
    }
    return [];
  }, [configsData, device]);

  const getWarningMessageConent = useMemo(() => {
    if (setupMode === DeviceModalSetupMode.AUTO_CONFIG) {
      return parse(LL.modals.addDevice.web.steps.config.helpers.warningAutoMode());
    }
    return parse(LL.modals.addDevice.web.steps.config.helpers.warningManualMode());
  }, [LL.modals.addDevice.web.steps.config.helpers, setupMode]);

  const getExpandCardExtras = useMemo(() => {
    return (
      <Select
        selected={selectedConfigOption}
        options={getSelectOptions}
        onChange={(o) => {
          if (!Array.isArray(o)) {
            setSelectedConfigOption(o);
          }
        }}
        multi={false}
        searchable={false}
        sizeVariant={SelectSizeVariant.STANDARD}
        loading={standAloneMode && loadingConfig}
      />
    );
  }, [getSelectOptions, loadingConfig, selectedConfigOption, standAloneMode]);

  // init select on mount
  useEffect(() => {
    if (configsData && configsData.length && isUndefined(selectedConfigOption)) {
      setSelectedConfigOption(configToSelectOption(configsData[0]));
    }
    if (standAloneMode && device) {
      setSelectedConfigOption(networkInfoToSelectOption(device.networks[0]));
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // set correct config after selected, only setup mode, standalone uses query
  useEffect(() => {
    if (!standAloneMode && configsData && selectedConfigOption) {
      const config = configsData.find((c) => c.network_id === selectedConfigOption.value);
      if (config) {
        setSelectedConfig(config.config);
      }
    }
  }, [configsData, selectedConfigOption, standAloneMode]);

  return (
    <>
      <MessageBox type={MessageBoxType.WARNING}>{getWarningMessageConent}</MessageBox>
      <Input
        outerLabel={LL.modals.addDevice.web.steps.config.inputNameLabel()}
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
