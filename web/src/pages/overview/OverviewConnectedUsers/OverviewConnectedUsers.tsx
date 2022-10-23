import './style.scss';

import { useMemo } from 'react';

import { NetworkUserStats, OverviewLayoutType } from '../../../shared/types';
import { getMaxDeviceStats } from '../helpers/stats';
import { useOverviewStore } from '../hooks/store/useOverviewStore';
import { UserConnectionCard } from './UserConnectionCard/UserConnectionCard';
import { UserConnectionListItem } from './UserConnectionListItem/UserConnectionListItem';

interface Props {
  stats?: NetworkUserStats[];
}

export const OverviewConnectedUsers = ({ stats }: Props) => {
  const viewMode = useOverviewStore((state) => state.viewMode);
  const getContentClassName = useMemo(() => {
    const rest = ['connected-users'];
    switch (viewMode) {
      case OverviewLayoutType.GRID:
        rest.push('grid');
        break;
      case OverviewLayoutType.LIST:
        rest.push('list');
        break;
    }
    return rest.join(' ');
  }, [viewMode]);
  const dataMax = stats ? getMaxDeviceStats(stats) : undefined;

  const renderedStats = useMemo(() => {
    if (!stats || !stats.length) {
      return null;
    }

    if (viewMode === OverviewLayoutType.GRID) {
      return stats.map((userStats) => (
        <UserConnectionCard
          key={userStats.user.username}
          data={userStats}
          dataMax={dataMax}
        />
      ));
    }

    return <RenderUserList data={stats} dataMax={dataMax} />;
  }, [stats, viewMode, dataMax]);

  return (
    <div className="overview-connected-users">
      <header>
        <h2>Connected users</h2>
      </header>
      {!stats || !stats.length ? (
        <p className="no-data-text">Currently there are no connected users</p>
      ) : null}
      <div className={getContentClassName}>{renderedStats}</div>
    </div>
  );
};

interface RenderUserListProps {
  data: NetworkUserStats[];
  dataMax: number | undefined;
}

const RenderUserList = ({ data, dataMax }: RenderUserListProps) => {
  return (
    <>
      <div className="headers">
        <div className="header">
          <span>User name</span>
        </div>
        <div className="header">
          <span>Device</span>
        </div>
        <div className="header">
          <span>Connected</span>
        </div>
        <div className="header">
          <span>Device location</span>
        </div>
        {/* <div className="header">
          <span>Connections</span>
        </div> */}
        <div className="header">
          <span>Network usage</span>
        </div>
      </div>
      <div className="users-list">
        {data.map((userStats) => (
          <UserConnectionListItem
            key={userStats.user.username}
            data={userStats}
            dataMax={dataMax}
          />
        ))}
      </div>
    </>
  );
};
