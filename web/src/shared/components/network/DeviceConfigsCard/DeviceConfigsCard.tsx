import './style.scss';

import { useQuery } from '@tanstack/react-query';
import { isUndefined } from 'lodash-es';
import { useCallback, useMemo, useState } from 'react';
import QRCode from 'react-qr-code';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { ActionButton } from '../../../defguard-ui/components/Layout/ActionButton/ActionButton';
import { ActionButtonVariant } from '../../../defguard-ui/components/Layout/ActionButton/types';
import { ExpandableCard } from '../../../defguard-ui/components/Layout/ExpandableCard/ExpandableCard';
import { LoaderSpinner } from '../../../defguard-ui/components/Layout/LoaderSpinner/LoaderSpinner';
import { Select } from '../../../defguard-ui/components/Layout/Select/Select';
import {
  SelectOption,
  SelectSelectedValue,
  SelectSizeVariant,
} from '../../../defguard-ui/components/Layout/Select/types';
import useApi from '../../../hooks/useApi';
import { useClipboard } from '../../../hooks/useClipboard';
import { QueryKeys } from '../../../queries';
import { GetDeviceConfigRequest } from '../../../types';
import { downloadWGConfig } from '../../../utils/downloadWGConfig';
import { DeviceConfigsCardNetworkInfo } from './types';

type Props = {
  deviceId: number;
  deviceName: string;
  userId: number;
  // if added it will insert it into configs instead of insering public key
  privateKey?: string;
  publicKey: string;
  networks: DeviceConfigsCardNetworkInfo[];
};

/*Expandable card variant that shows wireguard configs in qrcode form and allows for copy and download of them*/
export const DeviceConfigsCard = ({
  deviceId,
  deviceName,
  privateKey,
  publicKey,
  networks,
}: Props) => {
  const { writeToClipboard } = useClipboard();
  const { LL } = useI18nContext();
  const localLL = LL.components.deviceConfigsCard;
  const {
    device: { downloadDeviceConfig },
  } = useApi();

  const [selectedConfig, setSelectedConfig] = useState<string | undefined>();
  const [selectedNetwork, setSelectedNetwork] = useState<number>(networks[0].networkId);

  const queryParams = useMemo((): GetDeviceConfigRequest => {
    return {
      device_id: deviceId,
      network_id: selectedNetwork,
    };
  }, [selectedNetwork, deviceId]);

  const { isLoading: loadingConfig } = useQuery(
    [QueryKeys.FETCH_DEVICE_CONFIG, queryParams],
    () => downloadDeviceConfig(queryParams as GetDeviceConfigRequest),
    {
      enabled: !!queryParams,
      onSuccess: (res) => {
        if (privateKey) {
          setSelectedConfig(res.replace('YOUR_PRIVATE_KEY', privateKey));
        } else {
          setSelectedConfig(res.replace('YOUR_PRIVATE_KEY', publicKey));
        }
      },
    },
  );

  const getSelectOptions = useMemo((): SelectOption<number>[] => {
    return networks.map((n) => ({
      value: n.networkId,
      label: n.networkName,
      key: n.networkId,
    }));
  }, [networks]);

  const renderSelected = useCallback(
    (selected: number): SelectSelectedValue => {
      const option = getSelectOptions.find((o) => o.value === selected);
      if (!option) throw Error("Selected value doesn't exist");
      return {
        key: option.key,
        displayValue: option.label,
      };
    },
    [getSelectOptions],
  );

  const getExpandCardExtras = useMemo(() => {
    return (
      <Select
        renderSelected={renderSelected}
        selected={selectedNetwork}
        options={getSelectOptions}
        searchable={false}
        sizeVariant={SelectSizeVariant.SMALL}
        loading={loadingConfig}
        onChangeSingle={(networkId) => {
          setSelectedNetwork(networkId);
        }}
      />
    );
  }, [loadingConfig, getSelectOptions, selectedNetwork, renderSelected]);

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
              LL.components.deviceConfigsCard.messages.copyConfig(),
            );
          }
        }}
        disabled={isUndefined(selectedConfig)}
      />,
      <ActionButton
        variant={ActionButtonVariant.DOWNLOAD}
        key={3}
        onClick={() => {
          if (selectedConfig) {
            downloadWGConfig(selectedConfig, `${deviceName}.config`);
          }
        }}
        disabled={isUndefined(selectedConfig)}
      />,
    ];
  }, [
    deviceName,
    LL.components.deviceConfigsCard.messages,
    selectedConfig,
    writeToClipboard,
  ]);

  return (
    <ExpandableCard
      className="device-configs-card"
      title={localLL.cardTitle()}
      actions={expandableCardActions}
      topExtras={getExpandCardExtras}
      expanded
    >
      {selectedConfig && <QRCode value={selectedConfig} size={250} />}
      {isUndefined(selectedConfig) && <LoaderSpinner size={250} />}
    </ExpandableCard>
  );
};
