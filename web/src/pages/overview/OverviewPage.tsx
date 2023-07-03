import './style.scss';

import { useQuery } from '@tanstack/react-query';
import { isUndefined } from 'lodash-es';
import { useEffect, useMemo } from 'react';
import { useNavigate } from 'react-router';
import { useBreakpoint } from 'use-breakpoint';

import { useI18nContext } from '../../i18n/i18n-react';
import LoaderSpinner from '../../shared/components/layout/LoaderSpinner/LoaderSpinner';
import NoData from '../../shared/components/layout/NoData/NoData';
import { PageContainer } from '../../shared/components/layout/PageContainer/PageContainer';
import { GatewaysStatus } from '../../shared/components/network/GatewaysStatus/GatewaysStatus';
import { deviceBreakpoints } from '../../shared/constants';
import useApi from '../../shared/hooks/useApi';
import { QueryKeys } from '../../shared/queries';
import { NetworkUserStats, OverviewLayoutType } from '../../shared/types';
import { sortByDate } from '../../shared/utils/sortByDate';
import { useWizardStore } from '../wizard/hooks/useWizardStore';
import { getNetworkStatsFilterValue } from './helpers/stats';
import { useOverviewStore } from './hooks/store/useOverviewStore';
import { OverviewConnectedUsers } from './OverviewConnectedUsers/OverviewConnectedUsers';
import { OverviewHeader } from './OverviewHeader/OverviewHeader';
import { OverviewStats } from './OverviewStats/OverviewStats';

const STATUS_REFETCH_TIMEOUT = 15 * 1000;

export const OverviewPage = () => {
  const navigate = useNavigate();
  const { breakpoint } = useBreakpoint(deviceBreakpoints);
  const viewMode = useOverviewStore((state) => state.viewMode);
  const setOverViewStore = useOverviewStore((state) => state.setState);
  const statsFilter = useOverviewStore((state) => state.statsFilter);
  const selectedNetworkId = useOverviewStore((state) => state.selectedNetworkId);
  const resetWizard = useWizardStore((state) => state.resetState);
  const { LL } = useI18nContext();

  const {
    network: { getNetworks, getUsersStats, getNetworkStats },
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
        }
      },
    }
  );

  const { data: networkStats } = useQuery(
    [QueryKeys.FETCH_NETWORK_STATS, statsFilter, selectedNetworkId],
    () =>
      getNetworkStats({
        from: getNetworkStatsFilterValue(statsFilter),
        id: selectedNetworkId,
      }),
    {
      refetchOnWindowFocus: false,
      refetchInterval: STATUS_REFETCH_TIMEOUT,
      enabled: !isUndefined(selectedNetworkId),
    }
  );

  const { data: networkUsersStats, isLoading: userStatsLoading } = useQuery(
    [QueryKeys.FETCH_NETWORK_USERS_STATS, statsFilter, selectedNetworkId],
    () =>
      getUsersStats({
        from: getNetworkStatsFilterValue(statsFilter),
        id: selectedNetworkId,
      }),
    {
      enabled: !isUndefined(statsFilter) && !isUndefined(selectedNetworkId),
      refetchOnWindowFocus: false,
      refetchInterval: STATUS_REFETCH_TIMEOUT,
    }
  );

  const getNetworkUsers = useMemo(() => {
    let res: NetworkUserStats[] = [];
    if (!isUndefined(networkUsersStats)) {
      res = sortByDate(
        networkUsersStats,
        (i) => {
          const devices = sortByDate(i.devices, (d) => d.connected_at, false);
          return devices[0].connected_at;
        },
        false
      );
    }
    return res;
  }, [networkUsersStats]);

  useEffect(() => {
    if (breakpoint === 'mobile' && viewMode === OverviewLayoutType.LIST) {
      setOverViewStore({ viewMode: OverviewLayoutType.GRID });
    }
    if (breakpoint === 'tablet' && viewMode === OverviewLayoutType.GRID) {
      setOverViewStore({ viewMode: OverviewLayoutType.LIST });
    }
  }, [viewMode, breakpoint, setOverViewStore]);

  return (
    <>
      <PageContainer id="network-overview-page">
        <OverviewHeader loading={networksLoading} />
        <GatewaysStatus networkId={selectedNetworkId} />
        {networkStats && networkUsersStats && (
          <OverviewStats usersStats={networkUsersStats} networkStats={networkStats} />
        )}
        <div className="bottom-row">
          {userStatsLoading ? (
            <div className="stats-loader">
              <LoaderSpinner size={180} />
            </div>
          ) : getNetworkUsers.length > 0 ? (
            <OverviewConnectedUsers stats={getNetworkUsers} />
          ) : (
            <NoData customMessage={LL.networkOverview.stats.gatewayDisconnected()} />
          )}
        </div>
      </PageContainer>
    </>
  );
};
