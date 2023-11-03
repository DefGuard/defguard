import './style.scss';

import classNames from 'classnames';
import dayjs from 'dayjs';
import utc from 'dayjs/plugin/utc';
import { TargetAndTransition } from 'framer-motion';
import { isUndefined, orderBy } from 'lodash-es';
import { useMemo, useState } from 'react';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import IconClip from '../../../../../shared/components/svg/IconClip';
import SvgIconCollapse from '../../../../../shared/components/svg/IconCollapse';
import SvgIconExpand from '../../../../../shared/components/svg/IconExpand';
import { ColorsRGB } from '../../../../../shared/constants';
import { Badge } from '../../../../../shared/defguard-ui/components/Layout/Badge/Badge';
import { Card } from '../../../../../shared/defguard-ui/components/Layout/Card/Card';
import { DeviceAvatar } from '../../../../../shared/defguard-ui/components/Layout/DeviceAvatar/DeviceAvatar';
import { EditButton } from '../../../../../shared/defguard-ui/components/Layout/EditButton/EditButton';
import { EditButtonOption } from '../../../../../shared/defguard-ui/components/Layout/EditButton/EditButtonOption';
import { EditButtonOptionStyleVariant } from '../../../../../shared/defguard-ui/components/Layout/EditButton/types';
import { Label } from '../../../../../shared/defguard-ui/components/Layout/Label/Label';
import { NoData } from '../../../../../shared/defguard-ui/components/Layout/NoData/NoData';
import { useUserProfileStore } from '../../../../../shared/hooks/store/useUserProfileStore';
import { Device, DeviceNetworkInfo } from '../../../../../shared/types';
import { sortByDate } from '../../../../../shared/utils/sortByDate';
import { useDeleteDeviceModal } from '../hooks/useDeleteDeviceModal';
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

  // first, order by last_connected_at then if not preset, by network_id
  const orderedLocations = useMemo((): DeviceNetworkInfo[] => {
    const connected = device.networks.filter(
      (network) => !isUndefined(network.last_connected_at),
    );

    const neverConnected = device.networks.filter((network) =>
      isUndefined(network.last_connected_at),
    );

    const connectedSorted = sortByDate(
      connected,
      (n) => n.last_connected_at as string,
      true,
    );
    const neverConnectedSorted = orderBy(neverConnected, ['network_id'], ['desc']);

    return [...connectedSorted, ...neverConnectedSorted];
  }, [device.networks]);

  const latestLocation = orderedLocations.length ? orderedLocations[0] : undefined;

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
          <DeviceAvatar deviceId={Number(device.id)} active={false} />
          <h3 data-testid="device-name">{device.name}</h3>
        </header>
        <div className="section-content">
          <div>
            <Label>{LL.userPage.devices.card.labels.publicIP()}</Label>
            {latestLocation?.last_connected_ip && (
              <p data-testid="device-last-connected-from">
                {latestLocation.last_connected_ip}
              </p>
            )}
            {!latestLocation?.last_connected_ip && (
              <NoData customMessage={LL.userPage.devices.card.labels.noData()} />
            )}
          </div>
          <div>
            <Label>{LL.userPage.devices.card.labels.connectedThrough()}</Label>
            {latestLocation && latestLocation.last_connected_at && (
              <p>{latestLocation?.network_name}</p>
            )}
            {!latestLocation?.last_connected_at && (
              <NoData customMessage={LL.userPage.devices.card.labels.noData()} />
            )}
          </div>
          <div>
            <Label>{LL.userPage.devices.card.labels.connectionDate()}</Label>
            {latestLocation && latestLocation.last_connected_at && (
              <p>{formatDate(latestLocation.last_connected_at)}</p>
            )}
            {!latestLocation?.last_connected_at && (
              <NoData customMessage={LL.userPage.devices.card.labels.noData()} />
            )}
          </div>
        </div>
      </section>
      <div className="locations">
        {orderedLocations.map((n) => (
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
          {last_connected_ip && (
            <p data-testid="device-last-connected-from">{last_connected_ip}</p>
          )}
          {!last_connected_ip && (
            <NoData customMessage={LL.userPage.devices.card.labels.noData()} />
          )}
        </div>
        <div>
          <Label>{LL.userPage.devices.card.labels.lastConnected()}</Label>
          {last_connected_at && (
            <p data-testid="device-last-connected-at">{formatDate(last_connected_at)}</p>
          )}
          {!last_connected_at && (
            <NoData customMessage={LL.userPage.devices.card.labels.noData()} />
          )}
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
