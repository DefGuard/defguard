import './style.scss';

import classNames from 'classnames';
import dayjs from 'dayjs';
import utc from 'dayjs/plugin/utc';
import { TargetAndTransition } from 'framer-motion';
import { useCallback, useMemo, useState } from 'react';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { AvatarBox } from '../../../../../shared/components/layout/AvatarBox/AvatarBox';
import Badge from '../../../../../shared/components/layout/Badge/Badge';
import { Card } from '../../../../../shared/components/layout/Card/Card';
import { DeviceAvatar } from '../../../../../shared/components/layout/DeviceAvatar/DeviceAvatar';
import { EditButton } from '../../../../../shared/components/layout/EditButton/EditButton';
import {
  EditButtonOption,
  EditButtonOptionStyleVariant,
} from '../../../../../shared/components/layout/EditButton/EditButtonOption';
import { Label } from '../../../../../shared/components/layout/Label/Label';
import { IconClip, IconCollapse, IconExpand } from '../../../../../shared/components/svg';
import { ColorsRGB } from '../../../../../shared/constants';
import { displayDate } from '../../../../../shared/helpers/displayDate';
import { useModalStore } from '../../../../../shared/hooks/store/useModalStore';
import { useUserProfileStore } from '../../../../../shared/hooks/store/useUserProfileStore';
import useApi from '../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../shared/hooks/useToaster';
import { Device, DeviceNetworkInfo } from '../../../../../shared/types';
import { downloadWGConfig } from '../../../../../shared/utils/downloadWGConfig';
import { sortByDate } from '../../../../../shared/utils/sortByDate';

dayjs.extend(utc);

const dateFormat = 'DD.MM.YYYY | HH:mm';

const formatDate = (date: string): string => {
  return dayjs.utc(date).format(dateFormat);
};

interface Props {
  device: Device;
}

export const DeviceCard = ({ device }: Props) => {
  const [hovered, setHovered] = useState(false);
  const [expanded, setExpanded] = useState(false);
  const { LL } = useI18nContext();
  const toaster = useToaster();
  const user = useUserProfileStore((state) => state.user);
  const setDeleteUserDeviceModal = useModalStore(
    (state) => state.setDeleteUserDeviceModal
  );
  const setModalsState = useModalStore((state) => state.setState);
  const {
    device: { downloadDeviceConfig },
  } = useApi();

  const handleDownload = useCallback(
    (network_id: number, network_name: string) => {
      downloadDeviceConfig({
        device_id: device.id,
        network_id,
      })
        .then((res) => {
          downloadWGConfig(
            res,
            `${device.name.toLowerCase().replace(' ', '')}-${network_name
              .toLowerCase()
              .replace(' ', '')}`
          );
        })
        .catch((err) => {
          toaster.error(LL.messages.clipboardError());
          console.error(err);
        });
    },
    [LL.messages, device.id, device.name, downloadDeviceConfig, toaster]
  );

  const renderDownloadConfigOptions = useMemo(() => {
    return device.networks.map((n) => (
      <EditButtonOption
        key={n.network_id}
        text={LL.userPage.devices.card.edit.downloadConfig({
          name: n.network_name,
        })}
        onClick={() => handleDownload(n.network_id, n.network_name)}
      />
    ));
  }, [LL.userPage.devices.card.edit, device.networks, handleDownload]);

  const cn = useMemo(
    () =>
      classNames('device-card', {
        expanded,
      }),
    [expanded]
  );

  const getContainerAnimate = useMemo((): TargetAndTransition => {
    const res: TargetAndTransition = {
      borderColor: ColorsRGB.White,
    };
    if (expanded || hovered) {
      res.borderColor = ColorsRGB.GrayBorder;
    }
    return res;
  }, [expanded, hovered]);

  const latestLocation = useMemo(() => {
    const sorted = sortByDate(
      device.networks.filter((network) => Boolean(network.last_connected_at)),
      (i) => i.last_connected_at as string,
      true
    );
    return sorted[0];
  }, [device.networks]);

  if (!user) return null;

  return (
    <Card
      className={cn}
      initial={false}
      animate={getContainerAnimate}
      onMouseOver={() => setHovered(true)}
      onMouseOut={() => setHovered(false)}
    >
      <section className="main-info">
        <header>
          <AvatarBox>
            <DeviceAvatar deviceId={Number(device.id)} />
          </AvatarBox>
          <h3 data-testid="device-name">{device.name}</h3>
        </header>
        <div className="section-content">
          <div>
            <Label>{LL.userPage.devices.card.labels.lastLocation()}</Label>
            <p data-testid="device-last-connected-from">
              {latestLocation.network_gateway_ip}
            </p>
          </div>
          <div>
            <Label>{LL.userPage.devices.card.labels.lastConnected()}</Label>
            <p>
              {latestLocation.last_connected_at &&
                formatDate(latestLocation.last_connected_at)}
            </p>
          </div>
          <div>
            <Label>{LL.userPage.devices.card.labels.assignedIp()}</Label>
            <p>{latestLocation.device_wireguard_ip}</p>
          </div>
        </div>
      </section>
      <div className="locations">
        {device.networks.map((n) => (
          <DeviceLocation key={n.network_id} network_info={n} />
        ))}
      </div>
      <div className="card-controls">
        <EditButton visible={true}>
          <EditButtonOption
            text={LL.userPage.devices.card.edit.edit()}
            onClick={() => {
              setModalsState({
                editUserDeviceModal: { visible: true, device: device },
              });
            }}
          />
          {renderDownloadConfigOptions}
          <EditButtonOption
            styleVariant={EditButtonOptionStyleVariant.WARNING}
            text={LL.userPage.devices.card.edit.delete()}
            onClick={() => setDeleteUserDeviceModal({ visible: true, device: device })}
          />
        </EditButton>
        <ExpandButton
          expanded={expanded}
          onClick={() => setExpanded((state) => !state)}
        />
      </div>
    </Card>
  );
};

type DeviceLocationProps = {
  network_info: DeviceNetworkInfo;
};

const DeviceLocation = ({
  network_info: {
    network_id,
    network_name,
    network_gateway_ip,
    last_connected_ip,
    last_connected_at,
    device_wireguard_ip,
  },
}: DeviceLocationProps) => {
  const { LL } = useI18nContext();
  return (
    <div className="location" data-testid={`device-location-id-${network_id}`}>
      <header>
        <IconClip />
        <h3 data-testid="device-location-name">{network_name}</h3>
        <Badge text={network_gateway_ip} />
      </header>
      <div className="section-content">
        <div>
          <Label>{LL.userPage.devices.card.labels.lastLocation()}</Label>
          <p data-testid="device-last-connected-from">{last_connected_ip}</p>
        </div>
        <div>
          <Label>{LL.userPage.devices.card.labels.lastConnected()}</Label>
          <p data-testid="device-last-connected-at">
            {last_connected_at && formatDate(last_connected_at)}
          </p>
        </div>
        <div>
          <Label>{LL.userPage.devices.card.labels.assignedIp()}</Label>
          <p data-testid="device-assigned-ip">{device_wireguard_ip}</p>
        </div>
      </div>
    </div>
  );
};

type ExpandButtonProps = {
  expanded: boolean;
  onClick: () => void;
};

const ExpandButton = ({ expanded, onClick }: ExpandButtonProps) => {
  return (
    <button className="device-card-expand" onClick={onClick}>
      {!expanded ? <IconExpand /> : <IconCollapse />}
    </button>
  );
};
