import './style.scss';

import dayjs from 'dayjs';
import utc from 'dayjs/plugin/utc';
import { floor } from 'lodash-es';
import { useMemo, useState } from 'react';

import Badge, {
  BadgeStyleVariant,
} from '../../../../shared/components/layout/Badge/Badge';
import {
  DeviceAvatar,
  DeviceAvatarVariants,
} from '../../../../shared/components/layout/DeviceAvatar/DeviceAvatar';
import IconButton from '../../../../shared/components/layout/IconButton/IconButton';
import {
  NetworkDirection,
  NetworkSpeed,
} from '../../../../shared/components/layout/NetworkSpeed/NetworkSpeed';
import UserInitials, {
  UserInitialsType,
} from '../../../../shared/components/layout/UserInitials/UserInitials';
import {
  IconPacketsIn,
  IconPacketsOut,
} from '../../../../shared/components/svg';
import SvgIconConnected from '../../../../shared/components/svg/IconConnected';
import SvgIconUserList from '../../../../shared/components/svg/IconUserList';
import SvgIconUserListElement from '../../../../shared/components/svg/IconUserListElement';
import SvgIconUserListExpanded from '../../../../shared/components/svg/IconUserListExpanded';
import { getUserFullName } from '../../../../shared/helpers/getUserFullName';
import { NetworkDeviceStats, NetworkUserStats } from '../../../../shared/types';
import { summarizeDeviceStats } from '../../helpers/stats';
import { NetworkUsageChart } from '../shared/components/NetworkUsageChart/NetworkUsageChart';
dayjs.extend(utc);

interface Props {
  data: NetworkUserStats;
  dataMax: number | undefined;
}

export const UserConnectionListItem = ({ data, dataMax }: Props) => {
  const [expanded, setExpanded] = useState(false);

  const getClassName = useMemo(() => {
    const res = ['user-connection-list-item'];
    if (expanded) {
      res.push('expanded');
    }
    return res.join(' ');
  }, [expanded]);

  return (
    <div className={getClassName}>
      <ExpandButton
        expanded={expanded}
        onExpand={() => setExpanded((state) => !state)}
      />
      <UserRow data={data} dataMax={dataMax} />
      {expanded &&
        data.devices.map((device) => (
          <DeviceRow data={device} key={device.id} dataMax={dataMax} />
        ))}
    </div>
  );
};

interface UserRowProps {
  data: NetworkUserStats;
  dataMax: number | undefined;
}

const UserRow = ({ data, dataMax }: UserRowProps) => {
  const getOldestDevice = useMemo(() => {
    const rankMap = data.devices.sort((a, b) => {
      const aDate = dayjs.utc(a.connected_at);
      const bDate = dayjs.utc(b.connected_at);
      return aDate.toDate().getTime() - bDate.toDate().getTime();
    });
    return rankMap[0];
  }, [data]);

  const getSummarizedDevicesStat = useMemo(
    () => summarizeDeviceStats(data.devices),
    [data.devices]
  );
  const downloadSummary = getSummarizedDevicesStat.reduce((sum, e) => {
    return sum + e.download;
  }, 0);

  const uploadSummary = getSummarizedDevicesStat.reduce((sum, e) => {
    return sum + e.upload;
  }, 0);

  return (
    <div className="user-row">
      <div className="user-name">
        <UserInitials
          first_name={data.user.first_name}
          last_name={data.user.last_name}
          type={UserInitialsType.BIG}
        />
        <span className="full-name">{getUserFullName(data.user)}</span>
      </div>
      <ActiveDevices data={data.devices} />
      <ConnectionTime connectedAt={getOldestDevice.connected_at} />
      <DeviceIps
        wireguardIp={getOldestDevice.wireguard_ip}
        publicIp={getOldestDevice.public_ip}
      />
      <div className="network-usage">
        <div className="network-usage-summary">
          <span className="transfer">
            <IconPacketsIn />
            <NetworkSpeed
              speedValue={downloadSummary}
              direction={NetworkDirection.DOWNLOAD}
              data-test="download"
            />
          </span>
          <span className="transfer">
            <IconPacketsOut />
            <NetworkSpeed
              speedValue={uploadSummary}
              direction={NetworkDirection.UPLOAD}
              data-test="upload"
            />
          </span>
        </div>
        <NetworkUsageChart
          data={getSummarizedDevicesStat}
          width={150}
          height={20}
          barSize={2}
          dataMax={dataMax}
        />
      </div>
    </div>
  );
};

interface DeviceRowProps {
  data: NetworkDeviceStats;
  dataMax: number | undefined;
}

const DeviceRow = ({ data, dataMax }: DeviceRowProps) => {
  const downloadSummary = data.stats.reduce((sum, e) => {
    return sum + e.download;
  }, 0);

  const uploadSummary = data.stats.reduce((sum, e) => {
    return sum + e.upload;
  }, 0);

  return (
    <div className="device-row">
      <div className="device-name">
        <SvgIconUserListElement />
        <DeviceAvatar
          styleVariant={DeviceAvatarVariants.GRAY_BOX}
          active={true}
        />
        <span className="name">{data.name}</span>
      </div>
      <div className="col-fill"></div>
      <ConnectionTime connectedAt={data.connected_at} />
      <DeviceIps publicIp={data.public_ip} wireguardIp={data.wireguard_ip} />
      <div className="network-usage">
        <div className="network-usage-summary">
          <span className="transfer">
            <IconPacketsIn />
            <NetworkSpeed
              speedValue={downloadSummary}
              direction={NetworkDirection.DOWNLOAD}
              data-test="download"
            />
          </span>
          <span className="transfer">
            <IconPacketsOut />
            <NetworkSpeed
              speedValue={uploadSummary}
              direction={NetworkDirection.UPLOAD}
              data-test="upload"
            />
          </span>
        </div>
        <NetworkUsageChart
          data={data.stats}
          width={150}
          height={20}
          barSize={2}
          dataMax={dataMax}
        />
      </div>
    </div>
  );
};

interface ActiveDevicesProps {
  data: NetworkDeviceStats[];
}

const ActiveDevices = ({ data }: ActiveDevicesProps) => {
  const activeDeviceCount = data.length;
  const showCount = useMemo(() => activeDeviceCount > 3, [activeDeviceCount]);
  const getCount = useMemo(() => activeDeviceCount - 2, [activeDeviceCount]);
  const getSliceEnd = useMemo(() => {
    if (activeDeviceCount > 3) {
      return 2;
    }
    return activeDeviceCount;
  }, [activeDeviceCount]);
  return (
    <div className="active-devices">
      {data.slice(0, getSliceEnd).map((device) => (
        <DeviceAvatar
          styleVariant={DeviceAvatarVariants.GRAY_BOX}
          active={true}
          key={device.id}
        />
      ))}
      {showCount && (
        <div className="count-box">
          <span className="count">+{getCount}</span>
        </div>
      )}
    </div>
  );
};

interface DeviceIpsProps {
  publicIp: string;
  wireguardIp: string;
}

const DeviceIps = ({ publicIp, wireguardIp }: DeviceIpsProps) => {
  return (
    <div className="device-ips">
      <Badge styleVariant={BadgeStyleVariant.STANDARD} text={publicIp} />
      <Badge styleVariant={BadgeStyleVariant.STANDARD} text={wireguardIp} />
    </div>
  );
};

// eslint-disable-next-line @typescript-eslint/no-unused-vars
const Connections = () => {
  return (
    <div className="connections">
      <UserInitials
        first_name="Z"
        last_name="K"
        type={UserInitialsType.SMALL}
      />
      <UserInitials
        first_name="A"
        last_name="P"
        type={UserInitialsType.SMALL}
      />
      <UserInitials
        first_name="R"
        last_name="O"
        type={UserInitialsType.SMALL}
      />
    </div>
  );
};

interface ConnectionTimeProps {
  connectedAt: string;
}

const ConnectionTime = ({ connectedAt }: ConnectionTimeProps) => {
  const getConnectionTime = useMemo(() => {
    const minutes = dayjs().diff(dayjs.utc(connectedAt), 'm');
    if (minutes > 60) {
      const hours = floor(minutes / 60);
      const res = [`${hours}h`];
      if (minutes % 60 > 0) {
        res.push(`${minutes % 60}m`);
      }
      return res.join(' ');
    }
    return `${minutes}m`;
  }, [connectedAt]);
  return (
    <div className="active-time">
      <SvgIconConnected />
      <span className="time">{getConnectionTime}</span>
    </div>
  );
};

interface ExpandButtonProps {
  expanded: boolean;
  onExpand: () => void;
}

const ExpandButton = ({ expanded, onExpand }: ExpandButtonProps) => {
  return (
    <IconButton onClick={() => onExpand()} className="blank expand-devices">
      {expanded ? <SvgIconUserListExpanded /> : <SvgIconUserList />}
    </IconButton>
  );
};
