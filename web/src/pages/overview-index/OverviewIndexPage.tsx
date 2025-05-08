import './style.scss';

import { useQuery } from '@tanstack/react-query';
import { sumBy } from 'lodash-es';
import { useCallback, useMemo } from 'react';

import { ExpandableSection } from '../../shared/components/Layout/ExpandableSection/ExpandableSection';
import { PageContainer } from '../../shared/components/Layout/PageContainer/PageContainer';
import { GatewaysStatus } from '../../shared/components/network/GatewaysStatus/GatewaysStatus';
import { Button } from '../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../shared/defguard-ui/components/Layout/Button/types';
import { isPresent } from '../../shared/defguard-ui/utils/isPresent';
import useApi from '../../shared/hooks/useApi';
import { useToaster } from '../../shared/hooks/useToaster';
import { Network, NetworkSpeedStats, WireguardNetworkStats } from '../../shared/types';
import { OverviewStats } from '../overview/OverviewStats/OverviewStats';

type OverviewIndexNetworkStats = Network & {
  stats: WireguardNetworkStats;
};

type NetworkSpeedTickData = {
  upload: number;
  download: number;
};

type SumMap = Record<string, NetworkSpeedTickData>;

const sumTickData = (
  a: NetworkSpeedTickData,
  b: NetworkSpeedTickData,
): NetworkSpeedTickData => {
  return {
    download: a.download + b.download,
    upload: a.upload + b.upload,
  };
};

const sumTransferSeries = (transferStats: NetworkSpeedStats[][]): NetworkSpeedStats[] => {
  const sumMap: SumMap = {};
  for (const stats of transferStats) {
    for (const tick of stats) {
      const tickValue = sumMap[tick.collected_at];
      if (isPresent(tickValue)) {
        sumMap[tick.collected_at] = sumTickData(tickValue, tick);
      } else {
        sumMap[tick.collected_at] = tick;
      }
    }
  }
  const res: NetworkSpeedStats[] = [];
  for (const sumMapKey of Object.keys(sumMap)) {
    const value = sumMap[sumMapKey];
    res.push({ collected_at: sumMapKey, download: value.download, upload: value.upload });
  }
  return res;
};

export const OverviewIndexPage = () => {
  const {
    network: { getNetworks, getNetworkStats },
  } = useApi();

  const toaster = useToaster();

  const query = useCallback(async () => {
    const res: OverviewIndexNetworkStats[] = [];
    const networks = await getNetworks();
    for (const network of networks) {
      const stats = await getNetworkStats({
        id: network.id,
      });
      res.push({
        ...network,
        stats,
      });
    }
    return res;
  }, [getNetworkStats, getNetworks]);

  const { data } = useQuery({
    queryKey: ['overview-index'],
    queryFn: query,
    refetchInterval: 10 * 1000,
  });

  const summaryData = useMemo(() => {
    if (!data) return undefined;
    const res: WireguardNetworkStats = {
      active_devices: sumBy(data, 'stats.active_devices'),
      active_users: sumBy(data, 'stats.active_users'),
      current_active_devices: sumBy(data, 'stats.current_active_devices'),
      current_active_users: sumBy(data, 'stats.current_active_users'),
      download: sumBy(data, 'stats.download'),
      upload: sumBy(data, 'stats.upload'),
      transfer_series: sumTransferSeries(
        data.map((network) => network.stats.transfer_series),
      ),
    };
    return res;
  }, [data]);

  return (
    <PageContainer id="overview-index">
      <div className="page-limited-content">
        <header>
          <h1>All locations overview</h1>
          <div className="controls"></div>
        </header>
        <ExpandableSection textAs="h2" text="All locations summary">
          {isPresent(summaryData) && <OverviewStats networkStats={summaryData} />}
        </ExpandableSection>
        {isPresent(data) &&
          data.map((network) => (
            <ExpandableSection
              className="network-section"
              textAs="h2"
              text={network.name}
              key={network.id}
            >
              <div className="top-track">
                <GatewaysStatus networkId={network.id} />
                <Button
                  size={ButtonSize.SMALL}
                  styleVariant={ButtonStyleVariant.LINK}
                  text="See Location Details"
                  onClick={() => {
                    toaster.warning('TODO: Need to refactor routing for this to work.');
                  }}
                />
              </div>
              <OverviewStats networkStats={network.stats} />
            </ExpandableSection>
          ))}
      </div>
    </PageContainer>
  );
};
