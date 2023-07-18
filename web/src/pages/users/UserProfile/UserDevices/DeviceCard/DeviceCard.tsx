import './style.scss';

import classNames from 'classnames';
import dayjs from 'dayjs';
import utc from 'dayjs/plugin/utc';
import { TargetAndTransition } from 'framer-motion';
import { isUndefined, orderBy } from 'lodash-es';
import { useMemo, useState } from 'react';

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
import { IconClip } from '../../../../../shared/components/svg';
import SvgIconCollapse from '../../../../../shared/components/svg/IconCollapse';
import SvgIconExpand from '../../../../../shared/components/svg/IconExpand';
import { ColorsRGB } from '../../../../../shared/constants';
import { useUserProfileStore } from '../../../../../shared/hooks/store/useUserProfileStore';
import { Device, DeviceNetworkInfo } from '../../../../../shared/types';
import { sortByDate } from '../../../../../shared/utils/sortByDate';
import { useDeleteDeviceModal } from '../hooks/useDeleteDeviceModal';
import { DeviceModalSetupMode, useDeviceModal } from '../hooks/useDeviceModal';
import { useEditDeviceModal } from '../hooks/useEditDeviceModal';

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
  const user = useUserProfileStore((state) => state.userProfile);
  const setDeleteDeviceModal = useDeleteDeviceModal((state) => state.setState);
  const setEditDeviceModal = useEditDeviceModal((state) => state.setState);
  const openDeviceModal = useDeviceModal((state) => state.open);

  const cn = useMemo(
    () =>
      classNames('device-card', {
        expanded,
      }),
    [expanded],
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

  const sortedLocations = useMemo(() => {
    let sortByDateAvailable = true;
    device.networks.forEach((n) => {
      if (!n.last_connected_at) {
        sortByDateAvailable = false;
      }
    });
    if (sortByDateAvailable) {
      const sorted = sortByDate(
        device.networks.filter((network) => Boolean(network.last_connected_at)),
        (i) => i.last_connected_at as string,
        true,
      );
      return sorted;
    } else {
      return orderBy(device.networks, ['network_id'], ['desc']);
    }
  }, [device.networks]);

  const latestLocation = useMemo(() => {
    if (sortedLocations.length) {
      return sortedLocations[0];
    }
    return undefined;
  }, [sortedLocations]);

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
              {latestLocation?.last_connected_ip}
            </p>
          </div>
          <div>
            <Label>{LL.userPage.devices.card.labels.lastConnected()}</Label>
            <p>
              {!isUndefined(latestLocation) &&
                !isUndefined(latestLocation.last_connected_at) &&
                formatDate(latestLocation.last_connected_at)}
            </p>
          </div>
          <div>
            <Label>{LL.userPage.devices.card.labels.assignedIp()}</Label>
            <p>{latestLocation?.device_wireguard_ip}</p>
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
              setEditDeviceModal({
                visible: true,
                device: device,
              });
            }}
          />
          <EditButtonOption
            styleVariant={EditButtonOptionStyleVariant.STANDARD}
            text={LL.userPage.devices.card.edit.showConfigurations()}
            onClick={() =>
              openDeviceModal({
                visible: true,
                currentStep: 1,
                setupMode: DeviceModalSetupMode.MANUAL_CONFIG,
                device: device,
              })
            }
          />
          <EditButtonOption
            styleVariant={EditButtonOptionStyleVariant.WARNING}
            text={LL.userPage.devices.card.edit.delete()}
            onClick={() =>
              setDeleteDeviceModal({
                visible: true,
                device: device,
              })
            }
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
        <div className="info-wrapper">
          <h3 data-testid="device-location-name">{network_name}</h3>
          {!isUndefined(network_gateway_ip) && <Badge text={network_gateway_ip} />}
        </div>
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
      {expanded ? <SvgIconCollapse /> : <SvgIconExpand />}
    </button>
  );
};
