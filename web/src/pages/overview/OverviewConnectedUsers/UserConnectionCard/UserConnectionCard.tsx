import './style.scss';

import classNames from 'classnames';
import dayjs from 'dayjs';
import { motion } from 'framer-motion';
import { sumBy } from 'lodash-es';
import { useCallback, useEffect, useMemo, useState } from 'react';
import AutoSizer from 'react-virtualized-auto-sizer';
import { timer } from 'rxjs';

import { useI18nContext } from '../../../../i18n/i18n-react';
import SvgIconClip from '../../../../shared/components/svg/IconClip';
import SvgIconCollapse from '../../../../shared/components/svg/IconCollapse';
import SvgIconConnected from '../../../../shared/components/svg/IconConnected';
import SvgIconExpand from '../../../../shared/components/svg/IconExpand';
import { Badge } from '../../../../shared/defguard-ui/components/Layout/Badge/Badge';
import { BadgeStyleVariant } from '../../../../shared/defguard-ui/components/Layout/Badge/types';
import { DeviceAvatar } from '../../../../shared/defguard-ui/components/Layout/DeviceAvatar/DeviceAvatar';
import { NetworkSpeed } from '../../../../shared/defguard-ui/components/Layout/NetworkSpeed/NetworkSpeed';
import { NetworkDirection } from '../../../../shared/defguard-ui/components/Layout/NetworkSpeed/types';
import { UserInitials } from '../../../../shared/defguard-ui/components/Layout/UserInitials/UserInitials';
import { getUserFullName } from '../../../../shared/helpers/getUserFullName';
import {
  NetworkDeviceStats,
  NetworkUserStats,
  StandaloneDeviceStats,
} from '../../../../shared/types';
import { titleCase } from '../../../../shared/utils/titleCase';
import {
  summarizeDevicesStats,
  summarizeDeviceStats,
  summarizeUsersNetworkStats,
} from '../../helpers/stats';
import { NetworkUsageChart } from '../shared/components/NetworkUsageChart/NetworkUsageChart';
import { formatConnectionTime } from './formatConnectionTime';

type DeviceConnectionCardProps = {
  data: StandaloneDeviceStats;
};

export const StandaloneDeviceConnectionCard = ({ data }: DeviceConnectionCardProps) => {
  return (
    <div className="connected-user-card">
      <DeviceCardContent data={data} />
    </div>
  );
};

interface Props {
  data: NetworkUserStats;
}

export const UserConnectionCard = ({ data }: Props) => {
  const [expanded, setExpanded] = useState(false);

  const cn = useMemo(
    () =>
      classNames('connected-user-card', {
        expanded,
      }),
    [expanded],
  );

  return (
    <motion.div className={cn}>
      <MainCardContent data={data} />
      <div className="devices">
        {data?.devices &&
          data.devices.length > 0 &&
          expanded &&
          data.devices.map((device) => (
            <ExpandedDeviceCard key={device.id} data={device} />
          ))}
      </div>
      <ExpandButton expanded={expanded} onClick={() => setExpanded((state) => !state)} />
    </motion.div>
  );
};

const DeviceCardContent = (props: { data: StandaloneDeviceStats }) => {
  const { data } = props;

  const getSummarizedStats = useMemo(() => summarizeDeviceStats(data.stats), [data]);

  const totalUpload = useMemo(
    () => sumBy(getSummarizedStats, (s) => s.upload),
    [getSummarizedStats],
  );
  const totalDownload = useMemo(
    () => sumBy(getSummarizedStats, (s) => s.download),
    [getSummarizedStats],
  );

  return (
    <div className="user-info">
      <div className="upper">
        <DeviceAvatar deviceId={data.id} />
        <NameBox
          name={data.name}
          publicIp={data.public_ip}
          wireguardIp={data.wireguard_ip}
        />
      </div>
      <div className="lower device">
        <ConnectionTime connectedAt={data.connected_at} />
        <div className="network-usage-summary">
          <div className="network-usage-stats">
            <NetworkSpeed
              speedValue={totalDownload}
              direction={NetworkDirection.DOWNLOAD}
              data-testid="download"
            />
            <NetworkSpeed
              speedValue={totalUpload}
              direction={NetworkDirection.UPLOAD}
              data-testid="upload"
            />
          </div>
          <div className="chart">
            <AutoSizer>
              {({ height, width }) => (
                <NetworkUsageChart height={height} width={width} data={data.stats} />
              )}
            </AutoSizer>
          </div>
        </div>
      </div>
    </div>
  );
};

interface MainCardContentProps {
  data: NetworkUserStats;
}

const MainCardContent = ({ data }: MainCardContentProps) => {
  const getOldestDevice = useMemo(() => {
    const rankMap = data.devices.sort((a, b) => {
      const aDate = dayjs.utc(a.connected_at);
      const bDate = dayjs.utc(b.connected_at);
      return aDate.toDate().getTime() - bDate.toDate().getTime();
    });
    return rankMap[0];
  }, [data]);

  const getSummarizedStats = useMemo(
    () => summarizeDevicesStats(data.devices),
    [data.devices],
  );

  const getUserSummarizedStats = useMemo(
    () => summarizeUsersNetworkStats([data]),
    [data],
  );

  return (
    <div className="user-info">
      <div className="upper">
        <UserInitials
          first_name={data.user?.first_name}
          last_name={data.user?.last_name}
        />
        <NameBox
          name={getUserFullName(data.user)}
          publicIp={getOldestDevice.public_ip}
          wireguardIp={getOldestDevice.wireguard_ip}
        />
      </div>
      <div className="lower">
        <ConnectionTime connectedAt={getOldestDevice.connected_at} />
        <ActiveDevices data={data.devices} />
        <div className="network-usage-summary">
          <div className="network-usage-stats">
            <NetworkSpeed
              speedValue={getUserSummarizedStats.download}
              direction={NetworkDirection.DOWNLOAD}
              data-testid="download"
            />
            <NetworkSpeed
              speedValue={getUserSummarizedStats.upload}
              direction={NetworkDirection.UPLOAD}
              data-testid="upload"
            />
          </div>
          <div className="chart">
            <AutoSizer>
              {({ height, width }) => (
                <NetworkUsageChart
                  height={height}
                  width={width}
                  data={getSummarizedStats}
                />
              )}
            </AutoSizer>
          </div>
        </div>
      </div>
    </div>
  );
};

interface NameBoxProps {
  name: string;
  publicIp?: string;
  wireguardIp?: string;
}

const NameBox = ({ name, publicIp, wireguardIp }: NameBoxProps) => {
  return (
    <div className="name-box">
      <span className="name">{name}</span>
      {(publicIp || wireguardIp) && (
        <div className="lower">
          {publicIp !== undefined && publicIp.length > 0 && (
            <Badge type={BadgeStyleVariant.STANDARD} text={publicIp} />
          )}
          {wireguardIp !== undefined && wireguardIp.length > 0 && (
            <Badge type={BadgeStyleVariant.STANDARD} text={wireguardIp} />
          )}
        </div>
      )}
    </div>
  );
};

interface ConnectionTimeProps {
  connectedAt?: string;
}

const ConnectionTime = ({ connectedAt }: ConnectionTimeProps) => {
  const { LL } = useI18nContext();

  const [displayedTime, setDisplayedTime] = useState<string | undefined>();

  const updateConnectionTime = useCallback(() => {
    if (connectedAt) {
      setDisplayedTime(formatConnectionTime(connectedAt));
    }
    return LL.common.noData();
  }, [connectedAt, LL.common]);

  useEffect(() => {
    const interval = 60 * 1000;
    const sub = timer(0, interval).subscribe(() => {
      updateConnectionTime();
    });

    return () => {
      sub.unsubscribe();
    };
  }, [updateConnectionTime, connectedAt]);

  return (
    <div className="connection-time lower-box">
      <span className="label">{LL.connectedUsersOverview.userList.connected()}</span>
      <div className="time">
        <SvgIconConnected />
        <span data-testid="connection-time-value">{displayedTime}</span>
      </div>
    </div>
  );
};

interface ActiveDevicesProps {
  data: NetworkDeviceStats[];
}

const ActiveDevices = ({ data }: ActiveDevicesProps) => {
  const { LL } = useI18nContext();
  const activeDeviceCount = data.length;

  const showCount = useMemo(() => activeDeviceCount > 3, [activeDeviceCount]);

  const getCount = useMemo(() => activeDeviceCount - 2, [activeDeviceCount]);

  // trim data so only max 3 boxes are visible
  const getSliceEnd = useMemo(() => {
    if (activeDeviceCount > 3) {
      return 2;
    }
    return activeDeviceCount;
  }, [activeDeviceCount]);

  return (
    <div className="active-devices lower-box">
      <span className="label">{LL.connectedUsersOverview.userList.device()}</span>
      <div className="content-wrapper">
        {data.slice(0, getSliceEnd).map((device) => (
          <DeviceAvatar active={true} deviceId={device.id} key={device.id} />
        ))}
        {showCount && (
          <div className="count-box">
            <span className="count">+{getCount}</span>
          </div>
        )}
      </div>
    </div>
  );
};

interface DeviceAvatarBoxProps {
  id: number;
}

const DeviceAvatarBox = ({ id }: DeviceAvatarBoxProps) => {
  return (
    <div className="avatar-box">
      <DeviceAvatar deviceId={id} />
    </div>
  );
};

interface ExpandedDeviceCardProps {
  data: NetworkDeviceStats;
}

const ExpandedDeviceCard = ({ data }: ExpandedDeviceCardProps) => {
  const getSummarizedStats = useMemo(() => summarizeDevicesStats([data]), [data]);
  const downloadSummary = getSummarizedStats.reduce((sum, e) => {
    return sum + e.download;
  }, 0);

  const uploadSummary = getSummarizedStats.reduce((sum, e) => {
    return sum + e.upload;
  }, 0);

  return (
    <div className="device">
      <div className="upper">
        <SvgIconClip />
        <DeviceAvatarBox id={data.id} />
        <NameBox
          name={titleCase(data.name)}
          publicIp={data.public_ip}
          wireguardIp={data.wireguard_ip}
        />
      </div>
      <div className="lower">
        <ConnectionTime connectedAt={data.connected_at} />
        <div className="network-usage-summary">
          <div className="network-usage-stats">
            <NetworkSpeed
              speedValue={downloadSummary}
              direction={NetworkDirection.DOWNLOAD}
              data-testid="download"
            />
            <NetworkSpeed
              speedValue={uploadSummary}
              direction={NetworkDirection.UPLOAD}
              data-testid="upload"
            />
          </div>
          <div className="chart">
            <AutoSizer>
              {({ height, width }) => (
                <NetworkUsageChart data={data.stats} width={width} height={height} />
              )}
            </AutoSizer>
          </div>
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
    <button className="card-expand" onClick={onClick}>
      {expanded ? <SvgIconCollapse /> : <SvgIconExpand />}
    </button>
  );
};
