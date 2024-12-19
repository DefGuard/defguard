import './style.scss';

import { useQuery } from '@tanstack/react-query';
import { isUndefined, orderBy } from 'lodash-es';
import { useEffect, useMemo } from 'react';
import { useNavigate } from 'react-router';
import { useBreakpoint } from 'use-breakpoint';

import { PageContainer } from '../../shared/components/Layout/PageContainer/PageContainer';
import { GatewaysStatus } from '../../shared/components/network/GatewaysStatus/GatewaysStatus';
import { deviceBreakpoints } from '../../shared/constants';
import { LoaderSpinner } from '../../shared/defguard-ui/components/Layout/LoaderSpinner/LoaderSpinner';
import { NoData } from '../../shared/defguard-ui/components/Layout/NoData/NoData';
import useApi from '../../shared/hooks/useApi';
import { QueryKeys } from '../../shared/queries';
import { OverviewLayoutType } from '../../shared/types';
import { sortByDate } from '../../shared/utils/sortByDate';
import { useWizardStore } from '../wizard/hooks/useWizardStore';
import { getNetworkStatsFilterValue } from './helpers/stats';
import { useOverviewStore } from './hooks/store/useOverviewStore';
import { OverviewConnectedUsers } from './OverviewConnectedUsers/OverviewConnectedUsers';
import { StandaloneDeviceConnectionCard } from './OverviewConnectedUsers/UserConnectionCard/UserConnectionCard';
import { OverviewExpandable } from './OverviewExpandable/OverviewExpandable';
import { OverviewHeader } from './OverviewHeader/OverviewHeader';
import { OverviewStats } from './OverviewStats/OverviewStats';

const STATUS_REFETCH_TIMEOUT = 15 * 1000;

export const OverviewPage = () => {
  const navigate = useNavigate();
  const { breakpoint } = useBreakpoint(deviceBreakpoints);
  const setOverViewStore = useOverviewStore((state) => state.setState);
  const statsFilter = useOverviewStore((state) => state.statsFilter);
  const selectedNetworkId = useOverviewStore((state) => state.selectedNetworkId);
  const resetWizard = useWizardStore((state) => state.resetState);
  const viewMode = useOverviewStore((state) => state.viewMode);

  const {
    network: { getNetworks, getOverviewStats, getNetworkStats },
  } = useApi();

  const { isLoading: networksLoading } = useQuery(
    [QueryKeys.FETCH_NETWORKS],
    getNetworks,
    {
      onSuccess: (res) => {
        if (!res.length) {
          resetWizard();
          navigate('/admin/wizard', { replace: true });
        } else {
          setOverViewStore({ networks: res });
          const ids = res.map((n) => n.id);
          if (
            isUndefined(selectedNetworkId) ||
            (!isUndefined(selectedNetworkId) && !ids.includes(selectedNetworkId))
          ) {
            const oldestNetwork = orderBy(res, ['id'], ['asc'])[0];
            setOverViewStore({ selectedNetworkId: oldestNetwork.id });
          }
        }
      },
    },
  );

  const { data: networkStats } = useQuery(
    [QueryKeys.FETCH_NETWORK_STATS, statsFilter, selectedNetworkId],
    () =>
      getNetworkStats({
        from: getNetworkStatsFilterValue(statsFilter),
        id: selectedNetworkId as number,
      }),
    {
      refetchOnWindowFocus: false,
      refetchInterval: STATUS_REFETCH_TIMEOUT,
      enabled: !isUndefined(selectedNetworkId),
    },
  );

  const { data: overviewStats, isLoading: userStatsLoading } = useQuery(
    [QueryKeys.FETCH_NETWORK_USERS_STATS, statsFilter, selectedNetworkId],
    () =>
      getOverviewStats({
        from: getNetworkStatsFilterValue(statsFilter),
        id: selectedNetworkId as number,
      }),
    {
      enabled: !isUndefined(statsFilter) && !isUndefined(selectedNetworkId),
      refetchOnWindowFocus: false,
      refetchInterval: STATUS_REFETCH_TIMEOUT,
    },
  );

  const getNetworkUsers = useMemo(() => {
    if (overviewStats !== undefined) {
      const user = sortByDate(overviewStats.user_devices, (s) => {
        const fistDevice = sortByDate(s.devices, (d) => d.connected_at, false)[0];
        return fistDevice.connected_at;
      });
      const devices = sortByDate(
        overviewStats.network_devices.filter((d) => d.connected_at !== undefined),
        (d) => d.connected_at as string,
      );
      return {
        network_devices: devices,
        user_devices: user,
      };
    }
    return undefined;
  }, [overviewStats]);

  // FIXME: lock viewMode on grid for now
  useEffect(() => {
    if (viewMode !== OverviewLayoutType.GRID) {
      setOverViewStore({ viewMode: OverviewLayoutType.GRID });
    }
  }, [setOverViewStore, viewMode]);

  return (
    <>
      <PageContainer id="network-overview-page">
        <OverviewHeader loading={networksLoading} />
        {breakpoint === 'desktop' && !isUndefined(selectedNetworkId) && (
          <GatewaysStatus networkId={selectedNetworkId} />
        )}
        {networkStats && overviewStats && (
          <OverviewStats
            usersStats={overviewStats.user_devices}
            networkStats={networkStats}
          />
        )}
        <div className="bottom-row">
          {userStatsLoading && (
            <div className="stats-loader">
              <LoaderSpinner size={180} />
            </div>
          )}
          {!getNetworkUsers && !userStatsLoading && <NoData />}
          {!userStatsLoading &&
            getNetworkUsers &&
            getNetworkUsers.network_devices.length === 0 &&
            getNetworkUsers.user_devices.length === 0 && <NoData />}
          {!userStatsLoading &&
            getNetworkUsers &&
            getNetworkUsers.user_devices.length > 0 && (
              <OverviewExpandable title="Connected Users">
                <OverviewConnectedUsers stats={getNetworkUsers.user_devices} />
              </OverviewExpandable>
            )}
          {!userStatsLoading &&
            getNetworkUsers &&
            getNetworkUsers.network_devices.length > 0 && (
              <OverviewExpandable title="Connected Users">
                <div className="connection-cards">
                  <div className="connected-users grid">
                    {getNetworkUsers.network_devices.map((device) => (
                      <StandaloneDeviceConnectionCard data={device} key={device.id} />
                    ))}
                  </div>
                </div>
              </OverviewExpandable>
            )}
        </div>
      </PageContainer>
      {/* Modals */}
    </>
  );
};
