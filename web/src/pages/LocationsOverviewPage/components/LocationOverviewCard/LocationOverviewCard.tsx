import { type PropsWithChildren, useMemo, useState } from 'react';
import type {
  LocationStats,
  NetworkLocation,
  TransferStats,
} from '../../../../shared/api/types';
import './style.scss';
import { useQuery } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import { maxBy } from 'lodash-es';
import { m } from '../../../../paraglide/messages';
import api from '../../../../shared/api/api';
import { GatewaysStatusBadge } from '../../../../shared/components/GatewaysStatusBadge/GatewaysStatusBadge';
import { TransferChart } from '../../../../shared/components/TransferChart/TransferChart';
import { TransferText } from '../../../../shared/components/TransferText/TransferText';
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
import { mapTransferToChart } from '../../../../shared/utils/stats';

type Props = {
  location: NetworkLocation;
  statsPeriod?: number;
  showTop?: boolean;
  expanded?: boolean;
} & PropsWithChildren;

export const LocationOverviewCard = ({
  location,
  statsPeriod = 1,
  expanded: initialExpanded = false,
  showTop = false,
  children,
}: Props) => {
  const navigate = useNavigate({ from: '/vpn-overview/' });
  const [isOpen, setOpen] = useState(initialExpanded);
  const { data: stats } = useQuery({
    queryFn: () =>
      api.location.getLocationStats({
        id: location.id,
        from: statsPeriod,
      }),
    queryKey: [
      'network',
      location.id,
      'stats',
      {
        period: statsPeriod,
      },
    ],
    refetchInterval: 30_000,
    refetchOnReconnect: true,
    select: (response) => response.data,
    placeholderData: (prev) => prev,
  });

  if (!isPresent(stats)) return null;

  return (
    <OverviewCard data={stats} expanded={isOpen} statsPeriod={statsPeriod}>
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
            <GatewaysStatusBadge data={location.gateways ?? []} showDetails />
            <Divider orientation="vertical" spacing={ThemeSpacing.Lg} />
            <Button
              text={m.misc_details()}
              variant="outlined"
              onClick={() => {
                navigate({
                  to: '$locationId',
                  params: {
                    locationId: location.id.toString(),
                  },
                  search: (perv) => perv,
                });
              }}
            />
          </div>
        </div>
      )}
      {children}
    </OverviewCard>
  );
};

type OverviewCardProps = {
  data: LocationStats;
  expanded: boolean;
  statsPeriod: number;
  emptyStateTitle?: string;
  emptyStateSubtitle?: string;
} & PropsWithChildren;

export const OverviewCard = ({
  data: stats,
  statsPeriod,
  children,
  emptyStateSubtitle,
  emptyStateTitle,
  expanded = false,
}: OverviewCardProps) => {
  const dataEmpty = useMemo(() => {
    if (!isPresent(stats)) return false;
    return (
      stats.upload === 0 && stats.download === 0 && stats.transfer_series.length === 0
    );
  }, [stats]);

  return (
    <div className="location-overview-card">
      {children}
      <Fold open={expanded}>
        {isPresent(children) && <Divider spacing={ThemeSpacing.Md} />}
        {!isPresent(stats) ||
          (dataEmpty && (
            <>
              <SizedBox height={ThemeSpacing.Xl2} />
              <EmptyState
                icon="dashboard"
                title={emptyStateTitle ?? m.location_overview_empty_title()}
                subtitle={emptyStateSubtitle ?? m.location_overview_empty_subtitle()}
              />
              <SizedBox height={ThemeSpacing.Xl2} />
            </>
          ))}
        {isPresent(stats) && !dataEmpty && <Stats stats={stats} period={statsPeriod} />}
      </Fold>
    </div>
  );
};

type StatsProps = {
  stats: LocationStats;
  period: number;
};

const Stats = ({ stats, period }: StatsProps) => {
  return (
    <div className="stats-summary">
      <div className="stats-track">
        <StatsSegment
          icon="user"
          name={m.location_overview_stats_current_active_users()}
          count={stats.current_active_users}
          subCount={stats.current_active_user_devices}
          subCountLabel={m.location_overview_stats_total_user_devices()}
        />
        <StatsSegment
          icon="connected-devices"
          name={m.location_overview_stats_current_active_devices()}
          count={stats.current_active_user_devices + stats.current_active_network_devices}
        />
        <StatsSegment
          icon="user-active"
          name={m.location_overview_stats_active_users()}
          count={stats.active_users}
          subCountLabel={m.location_overview_stats_total_user_devices()}
          subCount={stats.active_user_devices}
        />
        <StatsSegment
          icon="devices-active"
          name={m.location_overview_stats_active_devices()}
          count={stats.active_user_devices + stats.active_network_devices}
        />
        <StatsSegment icon="activity" name={m.location_overview_stats_network_usage()}>
          <SizedBox height={8} />
          <div className="transfer-bar download">
            <div className="left">
              <Icon icon="arrow-big" rotationDirection="right" />
              <p>{m.location_overview_stats_network_usage_in()}</p>
            </div>
            <div className="right">
              <TransferText variant="download" data={stats.download} />
            </div>
          </div>
          <SizedBox height={ThemeSpacing.Sm} />
          <div className="transfer-bar upload">
            <div className="left">
              <Icon icon="arrow-big" rotationDirection="left" />
              <p>{m.location_overview_stats_network_usage_out()}</p>
            </div>
            <div className="right">
              <TransferText variant="upload" data={stats.upload} />
            </div>
          </div>
        </StatsSegment>
      </div>
      <TransferSection period={period} transfer={stats.transfer_series} />
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

type TransferSectionProps = {
  transfer: TransferStats[];
  period: number;
};

const TransferSection = ({ transfer, period }: TransferSectionProps) => {
  const maxDownload = useMemo(
    () => maxBy(transfer, (t) => t.download)?.download ?? 0,
    [transfer],
  );

  const maxUpload = useMemo(
    () => maxBy(transfer, (t) => t.upload)?.upload ?? 0,
    [transfer],
  );

  const chartMap = useMemo(() => mapTransferToChart(transfer), [transfer]);
  const activityPeriodLabel =
    period === 1
      ? m.location_overview_activity_period_one_hour()
      : m.location_overview_activity_period_hours({ period });

  return (
    <div className="transfer-section">
      <div className="top">
        <p>{activityPeriodLabel}</p>
        <div className="right">
          <p className="peak">{m.location_overview_peak()}</p>
          <TransferText data={maxDownload} variant="download" icon />
          <span className="sep">/</span>
          <TransferText data={maxUpload} variant="upload" icon />
        </div>
      </div>
      <TransferChart data={chartMap} showX height={50} />
    </div>
  );
};
