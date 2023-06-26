import './style.scss';

import dayjs from 'dayjs';
import utc from 'dayjs/plugin/utc';
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
import { displayDate } from '../../../../../shared/helpers/displayDate';
import { useModalStore } from '../../../../../shared/hooks/store/useModalStore';
import { useUserProfileStore } from '../../../../../shared/hooks/store/useUserProfileStore';
import useApi from '../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../shared/hooks/useToaster';
import { Device, DeviceNetworkInfo } from '../../../../../shared/types';
import { downloadWGConfig } from '../../../../../shared/utils/downloadWGConfig';
import classNames from 'classnames';
import { TargetAndTransition } from 'framer-motion';
import { ColorsRGB } from '../../../../../shared/constants';
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
    return device.network_info.map((n) => (
      <EditButtonOption
        key={n.network_id}
        text={LL.userPage.devices.card.edit.downloadConfig({
          name: n.network_name,
        })}
        onClick={() => handleDownload(n.network_id, n.network_name)}
      />
    ));
  }, [LL.userPage.devices.card.edit, device.network_info, handleDownload]);

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
    const sorted = sortByDate(device.network_info, (i) => i.last_connected_at, true);
    return sorted[0];
  }, [device.network_info]);

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
            <p>{formatDate(latestLocation.last_connected_at)}</p>
          </div>
          <div>
            <Label>{LL.userPage.devices.card.labels.assignedIp()}</Label>
            <p>{latestLocation.device_wireguard_ip}</p>
          </div>
        </div>
      </section>
      <div className="locations">
        {device.network_info.map((n) => (
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

const DeviceLocation = ({ network_info }: DeviceLocationProps) => {
  const { LL } = useI18nContext();
  return (
    <div
      className="location"
      data-testid={`device-location-id-${network_info.network_id}`}
    >
      <header>
        <IconClip />
        <h3 data-testid="device-location-name">{network_info.network_name}</h3>
        <Badge text={network_info.network_gateway_ip} />
      </header>
      <div className="section-content">
        <div>
          <Label>{LL.userPage.devices.card.labels.lastLocation()}</Label>
          <p data-testid="device-last-connected-from">{network_info.last_connected_ip}</p>
        </div>
        <div>
          <Label>{LL.userPage.devices.card.labels.lastConnected()}</Label>
          <p data-testid="device-last-connected-at">
            {formatDate(network_info.last_connected_at)}
          </p>
        </div>
        <div>
          <Label>{LL.userPage.devices.card.labels.assignedIp()}</Label>
          <p data-testid="device-assigned-ip">{network_info.device_wireguard_ip}</p>
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
