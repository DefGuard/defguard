import './style.scss';

import { NetworkUserStats } from '../../../shared/types';
import { UserConnectionCard } from './UserConnectionCard/UserConnectionCard';

interface Props {
  stats?: NetworkUserStats[];
}

export const OverviewConnectedUsers = ({ stats }: Props) => {
  return (
    <div className="connection-cards">
      {stats && stats.length > 0 && (
        <div className="connected-users grid">
          {stats.map((userStats) => (
            <UserConnectionCard key={userStats.user.username} data={userStats} />
          ))}
        </div>
      )}
    </div>
  );
};
