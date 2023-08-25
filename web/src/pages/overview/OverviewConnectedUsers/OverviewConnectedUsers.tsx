import './style.scss';

import { useMemo } from 'react';

import { useI18nContext } from '../../../i18n/i18n-react';
import { NetworkUserStats, OverviewLayoutType } from '../../../shared/types';
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
  const { LL } = useI18nContext();

  const renderedStats = useMemo(() => {
    if (!stats || !stats.length) {
      return null;
    }

    if (viewMode === OverviewLayoutType.GRID) {
      return stats.map((userStats) => (
        <UserConnectionCard key={userStats.user.username} data={userStats} />
      ));
    }

    return <RenderUserList data={stats} />;
  }, [stats, viewMode]);

  return (
    <div className="overview-connected-users">
      <header>
        <h2>{LL.connectedUsersOverview.pageTitle()}</h2>
      </header>
      {!stats || !stats.length ? (
        <p className="no-data-text">{LL.connectedUsersOverview.noUsersMessage()}</p>
      ) : null}
      <div className={getContentClassName}>{renderedStats}</div>
    </div>
  );
};

interface RenderUserListProps {
  data: NetworkUserStats[];
}

const RenderUserList = ({ data }: RenderUserListProps) => {
  const { LL } = useI18nContext();
  return (
    <>
      <div className="headers">
        <div className="header">
          <span>{LL.connectedUsersOverview.userList.username()}</span>
        </div>
        <div className="header">
          <span>{LL.connectedUsersOverview.userList.device()}</span>
        </div>
        <div className="header">
          <span>{LL.connectedUsersOverview.userList.connected()}</span>
        </div>
        <div className="header">
          <span>{LL.connectedUsersOverview.userList.deviceLocation()}</span>
        </div>
        <div className="header">
          <span>{LL.connectedUsersOverview.userList.networkUsage()}</span>
        </div>
      </div>
      <div className="users-list">
        {data.map((userStats) => (
          <UserConnectionListItem key={userStats.user.username} data={userStats} />
        ))}
      </div>
    </>
  );
};
