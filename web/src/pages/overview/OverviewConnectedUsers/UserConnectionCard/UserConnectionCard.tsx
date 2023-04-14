import './style.scss';

import dayjs from 'dayjs';
import utc from 'dayjs/plugin/utc';
import { motion } from 'framer-motion';
import { floor } from 'lodash-es';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { usePopper } from 'react-popper';

import { useI18nContext } from '../../../../i18n/i18n-react';
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
import SvgIconCancel from '../../../../shared/components/svg/IconCancel';
import SvgIconConnected from '../../../../shared/components/svg/IconConnected';
import SvgIconOpenModal from '../../../../shared/components/svg/IconOpenModal';
import SvgIconUserListElement from '../../../../shared/components/svg/IconUserListElement';
import { getUserFullName } from '../../../../shared/helpers/getUserFullName';
import { NetworkDeviceStats, NetworkUserStats } from '../../../../shared/types';
import { titleCase } from '../../../../shared/utils/titleCase';
import { summarizeDeviceStats, summarizeUsersNetworkStats } from '../../helpers/stats';
import { NetworkUsageChart } from '../shared/components/NetworkUsageChart/NetworkUsageChart';
dayjs.extend(utc);
interface Props {
  data: NetworkUserStats;
  dataMax: number | undefined;
}

export const UserConnectionCard = ({ data, dataMax }: Props) => {
  const [expanded, setExpanded] = useState(false);
  const [containerHovered, setContainerHovered] = useState(false);
  const [popperElement, setPopperElement] = useState<HTMLDivElement | null>(null);
  const pageElement = document.getElementById('network-overview-page');
  const [referenceElement, setReferenceElement] = useState<HTMLDivElement | null>(null);
  const {
    styles: popperStyles,
    attributes: popperAttributes,
    state,
  } = usePopper(referenceElement, popperElement, {
    placement: 'bottom',
    modifiers: [
      {
        name: 'offset',
        enabled: true,
        options: {
          offset: [0, -150],
        },
      },
    ],
  });
  const getClassName = useMemo(() => {
    const res = ['connected-user-card'];
    if (expanded) {
      res.push('expanded');
    }
    return res.join(' ');
  }, [expanded]);

  const getPopperClassName = useMemo(() => {
    const res = ['user-connection-card-popper-container'];
    if (expanded) {
      res.push('expanded');
    }
    if (state?.placement) {
      res.push(`placement-${state?.placement}`);
    }
    return res.join(' ');
  }, [expanded, state?.placement]);

  const checkClickOutside = useCallback(
    (event: MouseEvent) => {
      const popperRect = popperElement?.getBoundingClientRect();
      if (popperRect) {
        const start_x = popperRect?.x;
        const end_x = start_x + popperRect?.width;
        const start_y = popperRect?.y;
        const end_y = start_y + popperRect.height;
        const { clientX, clientY } = event;
        if (
          clientX < start_x ||
          clientX > end_x ||
          clientY < start_y ||
          clientY > end_y
        ) {
          setExpanded(false);
        }
      }
    },
    [popperElement]
  );

  useEffect(() => {
    if (expanded) {
      const element = document.body;
      element?.addEventListener('click', checkClickOutside);
      return () => {
        element?.removeEventListener('click', checkClickOutside);
      };
    }
  }, [checkClickOutside, expanded, state?.placement]);

  return (
    <>
      <motion.div
        className={getClassName}
        onHoverStart={() => setContainerHovered(true)}
        onHoverEnd={() => setContainerHovered(false)}
        ref={setReferenceElement}
      >
        {!expanded && <MainCardContent data={data} dataMax={dataMax} />}
        {containerHovered && !expanded && (
          <IconButton className="expand-button blank" onClick={() => setExpanded(true)}>
            <SvgIconOpenModal />
          </IconButton>
        )}
      </motion.div>
      {expanded && pageElement && (
        <div
          className={getPopperClassName}
          ref={setPopperElement}
          {...popperAttributes.popper}
          style={{ ...popperStyles.popper }}
        >
          <div className="user-info-wrapper">
            <MainCardContent data={data} dataMax={dataMax} />
          </div>
          <IconButton
            className="collapse-button blank"
            onClick={() => setExpanded(false)}
          >
            <SvgIconCancel />
          </IconButton>
          <div className="devices">
            {data.devices.map((device) => (
              <ExpandedDeviceCard key={device.id} data={device} dataMax={dataMax} />
            ))}
          </div>
        </div>
      )}
    </>
  );
};

interface MainCardContentProps {
  data: NetworkUserStats;
  dataMax: number | undefined;
}

const MainCardContent = ({ data, dataMax }: MainCardContentProps) => {
  const getOldestDevice = useMemo(() => {
    const rankMap = data.devices.sort((a, b) => {
      const aDate = dayjs.utc(a.connected_at);
      const bDate = dayjs.utc(b.connected_at);
      return aDate.toDate().getTime() - bDate.toDate().getTime();
    });
    return rankMap[0];
  }, [data]);
  const getSummarizedStats = useMemo(
    () => summarizeDeviceStats(data.devices),
    [data.devices]
  );
  const getUserSummarizedStats = useMemo(
    () => summarizeUsersNetworkStats([data]),
    [data]
  );

  return (
    <>
      <div className="upper">
        <UserInitials
          first_name={data.user?.first_name}
          last_name={data.user?.last_name}
          type={UserInitialsType.BIG}
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
          <NetworkUsageChart data={getSummarizedStats} dataMax={dataMax} />
        </div>
      </div>
    </>
  );
};

interface NameBoxProps {
  name: string;
  publicIp: string;
  wireguardIp: string;
}

const NameBox = ({ name, publicIp, wireguardIp }: NameBoxProps) => {
  return (
    <div className="name-box">
      <span className="name">{name}</span>
      <div className="lower">
        <Badge styleVariant={BadgeStyleVariant.STANDARD} text={publicIp} />
        <Badge styleVariant={BadgeStyleVariant.STANDARD} text={wireguardIp} />
      </div>
    </div>
  );
};

interface ConnectionTimeProps {
  connectedAt: string;
}

const ConnectionTime = ({ connectedAt }: ConnectionTimeProps) => {
  const { LL } = useI18nContext();
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
    <div className="connection-time lower-box">
      <span className="label">{LL.connectedUsersOverview.userList.connected()}</span>
      <div className="content-wrapper">
        <SvgIconConnected />
        <span data-testid="connection-time-value">{getConnectionTime}</span>
      </div>
    </div>
  );
};

// TODO: Reimplement when mesh network will be ready
// eslint-disable-next-line @typescript-eslint/no-unused-vars
const ActiveConnections = () => {
  return (
    <div className="active-connections lower-box">
      <span className="label">Connections:</span>
      <div className="content-wrapper">
        <UserInitials type={UserInitialsType.SMALL} first_name="Z" last_name="K" />
        <UserInitials type={UserInitialsType.SMALL} first_name="A" last_name="P" />
        <UserInitials type={UserInitialsType.SMALL} first_name="R" last_name="O" />
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
  const getCount = useMemo(() => 2 - activeDeviceCount, [activeDeviceCount]);
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
          <DeviceAvatar
            styleVariant={DeviceAvatarVariants.GRAY_BOX}
            active={true}
            deviceId={device.id}
            key={device.id}
          />
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
  dataMax: number | undefined;
}

const ExpandedDeviceCard = ({ data, dataMax }: ExpandedDeviceCardProps) => {
  const getSummarizedStats = useMemo(() => summarizeDeviceStats([data]), [data]);
  const downloadSummary = getSummarizedStats.reduce((sum, e) => {
    return sum + e.download;
  }, 0);

  const uploadSummary = getSummarizedStats.reduce((sum, e) => {
    return sum + e.upload;
  }, 0);

  return (
    <>
      <div className="expanded-device-card">
        <div className="upper">
          <SvgIconUserListElement />
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
            <NetworkUsageChart data={data.stats} width={180} dataMax={dataMax} />
          </div>
        </div>
      </div>
    </>
  );
};
