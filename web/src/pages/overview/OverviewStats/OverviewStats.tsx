import './style.scss';

import numbro from 'numbro';
import { forwardRef } from 'react';
import AutoSizer from 'react-virtualized-auto-sizer';
import useBreakpoint from 'use-breakpoint';

import {
  NetworkDirection,
  NetworkSpeed,
} from '../../../shared/components/layout/NetworkSpeed/NetworkSpeed';
import {
  Icon24HConnections,
  IconActiveConnections,
  IconPacketsIn,
  IconPacketsOut,
} from '../../../shared/components/svg';
import { deviceBreakpoints } from '../../../shared/constants';
import { NetworkUserStats, WireguardNetworkStats } from '../../../shared/types';
import { useOverviewStore } from '../hooks/store/useOverviewStore';
import { NetworkUsageChart } from '../OverviewConnectedUsers/shared/components/NetworkUsageChart/NetworkUsageChart';
import { useI18nContext } from '../../../i18n/i18n-react';

interface Props {
  usersStats?: NetworkUserStats[];
  networkStats: WireguardNetworkStats;
}

const formatStats = (value: number): string =>
  numbro(value).format({
    average: false,
    spaceSeparated: false,
    mantissa: 0,
  });

export const OverviewStats = forwardRef<HTMLDivElement, Props>(
  ({ networkStats }, ref) => {
    const { breakpoint } = useBreakpoint(deviceBreakpoints);
    const filterValue = useOverviewStore((state) => state.statsFilter);
		const {LL} = useI18nContext();
    return (
      <div className="overview-network-stats" ref={ref}>
        <div className="summary">
          <div className="info">
            <span className="info-title">{LL.networkOverview.stats.currentlyActiveUsers()}</span>
            <div className="content">
              <Icon24HConnections />
              <span className="info-value">
                {formatStats(networkStats.current_active_users)}
              </span>
            </div>
          </div>
          <div className="info">
            <span className="info-title">{LL.networkOverview.stats.currentlyActiveDevices()}</span>
            <div className="content">
              <IconActiveConnections />
              <span className="info-value">
                {formatStats(networkStats.current_active_devices)}
              </span>
            </div>
          </div>
          <div className="info">
            <span className="info-title">{LL.networkOverview.stats.activeUsersFilter({hour: filterValue})}</span>
            <div className="content">
              <Icon24HConnections />
              <span className="info-value">{networkStats.active_users}</span>
            </div>
          </div>
          <div className="info">
            <span className="info-title">{LL.networkOverview.stats.activeDevicesFilter({hour: filterValue})}</span>
            <div className="content">
              <Icon24HConnections />
              <span className="info-value">
                {formatStats(networkStats.active_devices)}
              </span>
            </div>
          </div>
          {breakpoint === 'desktop' && (
            <div className="info network-usage" data-test="network-usage">
              <span className="info-title">{LL.networkOverview.stats.totalTransfer()}</span>
              <div className="content">
                <div className="network-usage">
                  <span>
                    <IconPacketsIn /> {LL.networkOverview.stats.in()}
                  </span>
                  <NetworkSpeed
                    speedValue={networkStats.download}
                    direction={NetworkDirection.DOWNLOAD}
                    data-test="download"
                  />
                </div>
                <div className="network-usage">
                  <span>
                    <IconPacketsOut /> {LL.networkOverview.stats.out()}
                  </span>
                  <NetworkSpeed
                    speedValue={networkStats.upload}
                    direction={NetworkDirection.UPLOAD}
                    data-test="upload"
                  />
                </div>
              </div>
            </div>
          )}
        </div>
        <div className="activity-graph">
          <header>
            <h3>{LL.networkOverview.stats.activityIn({hour: filterValue})}</h3>
            <div className="peaks">
              <span>{LL.networkOverview.stats.totalTransfer()}</span>
              <div className="network-speed">
                <IconPacketsIn />
                <NetworkSpeed
                  speedValue={networkStats.download}
                  direction={NetworkDirection.DOWNLOAD}
                />
              </div>
              <div className="network-speed">
                <IconPacketsOut />
                <NetworkSpeed
                  speedValue={networkStats.upload}
                  direction={NetworkDirection.UPLOAD}
                />
              </div>
            </div>
          </header>
          <AutoSizer>
            {({ width }) => (
              <>
                {networkStats.transfer_series && (
                  <NetworkUsageChart
                    data={networkStats.transfer_series}
                    hideX={false}
                    height={35}
                    width={width}
                    barSize={2}
                  />
                )}
              </>
            )}
          </AutoSizer>
        </div>
      </div>
    );
  }
);
