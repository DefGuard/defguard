import { type PropsWithChildren, useState } from 'react';
import type { LocationStats, NetworkLocation } from '../../../../shared/api/types';
import './style.scss';
import { useQuery } from '@tanstack/react-query';
import api from '../../../../shared/api/api';
import { GatewaysStatusBadge } from '../../../../shared/components/GatewaysStatusBadge/GatewaysStatusBadge';
import { Badge } from '../../../../shared/defguard-ui/components/Badge/Badge';
import { BadgeVariant } from '../../../../shared/defguard-ui/components/Badge/types';
import { Button } from '../../../../shared/defguard-ui/components/Button/Button';
import { Divider } from '../../../../shared/defguard-ui/components/Divider/Divider';
import { EmptyState } from '../../../../shared/defguard-ui/components/EmptyState/EmptyState';
import { Fold } from '../../../../shared/defguard-ui/components/Fold/Fold';
import { Icon } from '../../../../shared/defguard-ui/components/Icon';
import type { IconKindValue } from '../../../../shared/defguard-ui/components/Icon/icon-types';
import { SizedBox } from '../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../../shared/defguard-ui/types';
import { isPresent } from '../../../../shared/defguard-ui/utils/isPresent';

type Props = {
  location: NetworkLocation;
  showTop?: boolean;
  expanded?: boolean;
};

export const LocationOverviewCard = ({
  location,
  expanded: initialExpanded = false,
  showTop = false,
}: Props) => {
  const [isOpen, setOpen] = useState(initialExpanded);
  const { data: stats } = useQuery({
    queryFn: () =>
      api.location.getLocationStats({
        id: location.id,
      }),
    queryKey: ['network', location.id, 'stats'],
    refetchInterval: 30_000,
    refetchOnMount: true,
    refetchOnReconnect: true,
    select: (response) => response.data,
  });

  return (
    <div className="location-overview-card">
      {showTop && (
        <div className="top">
          <div
            className="name"
            onClick={() => {
              setOpen((s) => !s);
            }}
          >
            <Icon icon="arrow-small" rotationDirection={isOpen ? 'down' : 'right'} />
            <p>{location.name}</p>
          </div>
          <div className="right">
            <GatewaysStatusBadge data={location.gateways} />
            <Divider orientation="vertical" spacing={ThemeSpacing.Lg} />
            <Button text="Details" variant="outlined" onClick={() => {}} />
          </div>
        </div>
      )}
      <Fold open={isOpen}>
        <Divider spacing={ThemeSpacing.Md} />
        {!isPresent(stats) && (
          <>
            <SizedBox height={ThemeSpacing.Xl2} />
            <EmptyState
              title={`This location doesn't have any data.`}
              subtitle={`The data for this location will be shown once the location is connected.`}
            />
            <SizedBox height={ThemeSpacing.Xl2} />
          </>
        )}
        {isPresent(stats) && <Stats stats={stats} />}
      </Fold>
    </div>
  );
};

type StatsProps = {
  stats: LocationStats;
};

const Stats = ({ stats }: StatsProps) => {
  return (
    <div className="stats-summary">
      <div className="stats-track">
        <StatsSegment
          icon="user"
          name="Currently active users"
          count={stats.current_active_users}
          subCount={stats.current_active_user_devices}
          subCountLabel="Total user devices"
        />
        <StatsSegment
          icon="connected-devices"
          name="Currently active devices"
          count={stats.current_active_user_devices + stats.current_active_network_devices}
        />
        <StatsSegment
          icon="user-active"
          name="Active users"
          count={stats.active_users}
          subCountLabel="Total user devices"
          subCount={stats.active_user_devices}
        />
        <StatsSegment
          icon="devices-active"
          name="Active devices in"
          count={stats.active_user_devices + stats.active_user_devices}
        />
        <StatsSegment icon="activity" name="Currently active users">
          <p>Transfer placeholder</p>
        </StatsSegment>
      </div>
      <div className="chart"></div>
    </div>
  );
};

type StatsSegmentProps = {
  name: string;
  icon: IconKindValue;
  count?: number;
  subCount?: number;
  subCountLabel?: string;
} & PropsWithChildren;

const StatsSegment = ({
  icon,
  name,
  count,
  subCount,
  subCountLabel,
  children,
}: StatsSegmentProps) => {
  return (
    <div className="stats-segment">
      <div className="name">
        <Icon icon={icon} />
        <p className="label">{name}</p>
      </div>
      {isPresent(count) && <p className="count">{count}</p>}
      {isPresent(subCount) && isPresent(subCountLabel) && (
        <div className="sub-count">
          <p className="label">{subCountLabel}:</p>
          <Badge variant={BadgeVariant.Default} text={subCount.toString()} />
        </div>
      )}
      {children}
    </div>
  );
};
