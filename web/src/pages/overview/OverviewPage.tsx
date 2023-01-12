import './style.scss';

import { useQuery, useQueryClient } from '@tanstack/react-query';
import { isUndefined } from 'lodash-es';
import { useEffect, useMemo } from 'react';
import { useNavigate } from 'react-router';
import useBreakpoint from 'use-breakpoint';

import Button, {
  ButtonStyleVariant,
} from '../../shared/components/layout/Button/Button';
import PageContainer from '../../shared/components/layout/PageContainer/PageContainer';
import { IconEditNetwork } from '../../shared/components/svg';
import { deviceBreakpoints } from '../../shared/constants';
import useApi from '../../shared/hooks/useApi';
import { QueryKeys } from '../../shared/queries';
import { NetworkUserStats, OverviewLayoutType } from '../../shared/types';
import { sortByDate } from '../../shared/utils/sortByDate';
import { useNetworkPageStore } from '../network/hooks/useNetworkPageStore';
import { getNetworkStatsFilterValue } from './helpers/stats';
import { useOverviewStore } from './hooks/store/useOverviewStore';
import { OverviewActivityStream } from './OverviewActivityStream/OverviewActivityStream';
import { OverviewConnectedUsers } from './OverviewConnectedUsers/OverviewConnectedUsers';
import { OverviewStats } from './OverviewStats/OverviewStats';
import { OverviewStatsFilterSelect } from './OverviewStatsFilterSelect/OverviewStatsFilterSelect';
import { OverviewViewSelect } from './OverviewViewSelect/OverviewViewSelect';
import { useI18nContext } from '../../i18n/i18n-react';

const STATUS_REFETCH_TIMEOUT = 15 * 1000;

export const OverviewPage = () => {
  const navigate = useNavigate();
  const { breakpoint } = useBreakpoint(deviceBreakpoints);
  const viewMode = useOverviewStore((state) => state.viewMode);
  const setOverViewStore = useOverviewStore((state) => state.setState);
  const statsFilter = useOverviewStore((state) => state.statsFilter);
  const setNetworkPageStore = useNetworkPageStore((state) => state.setState);
  const queryClient = useQueryClient();
  const { LL } = useI18nContext();

  const {
    network: { getNetworks, getUsersStats, getNetworkStats },
  } = useApi();

  const { data: networkStats } = useQuery(
    [QueryKeys.FETCH_NETWORK_STATS, statsFilter],
    () => getNetworkStats({ from: getNetworkStatsFilterValue(statsFilter) }),
    {
      onSuccess: async () => {
        setTimeout(
          () => queryClient.invalidateQueries([QueryKeys.FETCH_NETWORK_STATS]),
          STATUS_REFETCH_TIMEOUT
        );
      },
      refetchOnWindowFocus: false,
    }
  );

  const { data: networkUsersStats } = useQuery(
    [QueryKeys.FETCH_NETWORK_USERS_STATS, statsFilter],
    () => getUsersStats({ from: getNetworkStatsFilterValue(statsFilter) }),
    {
      enabled: !isUndefined(statsFilter),
      onSuccess: async () => {
        setTimeout(
          () =>
            queryClient.invalidateQueries([
              QueryKeys.FETCH_NETWORK_USERS_STATS,
            ]),
          STATUS_REFETCH_TIMEOUT
        );
      },
      refetchOnWindowFocus: false,
    }
  );

  const { data: networks, isLoading: networksLoading } = useQuery(
    [QueryKeys.FETCH_NETWORKS],
    getNetworks
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

  const handleNetworkAction = () => {
    if (networks && networks.length) {
      setNetworkPageStore({ network: networks[0] });
    }
    navigate('../network');
  };

  return (
    <>
      <PageContainer id="network-overview-page">
        {breakpoint !== 'desktop' && (
          <div className="mobile-options">
            <Button
              styleVariant={ButtonStyleVariant.STANDARD}
              text={
                isUndefined(networks) || !networks?.length
                    ? LL.networkOverview.controls.configureNetwork()
                    : LL.networkOverview.controls.editNetwork()
              }
              icon={<IconEditNetwork />}
              disabled={networksLoading}
              onClick={handleNetworkAction}
            />
            <OverviewStatsFilterSelect />
            <OverviewViewSelect />
          </div>
        )}
        {breakpoint === 'desktop' && (
          <header>
            <h1>{LL.networkOverview.pageTitle()}</h1>
            <div className="controls">
              <OverviewViewSelect />
              <OverviewStatsFilterSelect />
              <Button
                styleVariant={ButtonStyleVariant.STANDARD}
                text={
                  isUndefined(networks) || !networks?.length
                    ? LL.networkOverview.controls.configureNetwork()
                    : LL.networkOverview.controls.editNetwork()
                }
                icon={<IconEditNetwork />}
                disabled={networksLoading}
                onClick={handleNetworkAction}
              />
            </div>
          </header>
        )}
        {networkStats && networkUsersStats && (
          <OverviewStats
            usersStats={networkUsersStats}
            networkStats={networkStats}
          />
        )}
        <div className="bottom-row">
          <OverviewConnectedUsers stats={getNetworkUsers} />
          <OverviewActivityStream />
        </div>
      </PageContainer>
    </>
  );
};
