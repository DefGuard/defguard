import './style.scss';

import { useQuery } from '@tanstack/react-query';
import { useCallback, useMemo } from 'react';

import { ExpandableSection } from '../../shared/components/Layout/ExpandableSection/ExpandableSection';
import { PageContainer } from '../../shared/components/Layout/PageContainer/PageContainer';
import useApi from '../../shared/hooks/useApi';
import { Network, WireguardNetworkStats } from '../../shared/types';
import { sumBy } from 'lodash-es';

type OverviewIndexNetworkStats = Network & {
  stats: WireguardNetworkStats;
};

export const OverviewIndexPage = () => {
  const {
    network: { getNetworks, getNetworkStats },
  } = useApi();

  const query = useCallback(async () => {
    const res: OverviewIndexNetworkStats[] = [];
    const networks = await getNetworks();
    const stats: WireguardNetworkStats[] = [];
    for (const network of networks) {
      const stats = await getNetworkStats({
        id: network.id,
      });
      res.push({
        ...network,
        stats,
      });
    }
    return stats;
  }, [getNetworkStats, getNetworks]);

  const { data } = useQuery({
    queryKey: ['overview-index'],
    queryFn: query,
  });

  const summaryData = useMemo(() => {
    if (!data) return undefined;
    const res: WireguardNetworkStats = {
      active_devices: sumBy(data, 'active_devices'),
      active_users: sumBy(data, 'active_users'),
      current_active_devices: sumBy(data, 'current_active_devices'),
      current_active_users: sumBy(data, 'current_active_users'),
      download: sumBy(data, 'download'),
      upload: sumBy(data, 'upload'),
      transfer_series: [],
    };
    return res;
  }, [data]);

  return (
    <PageContainer id="overview-index">
      <div>
        <header>
          <h1>All locations overview</h1>
          <div className="controls"></div>
        </header>
        <ExpandableSection textAs="h2" text="All locations summary">
          <p>Summary indeed</p>
        </ExpandableSection>
      </div>
    </PageContainer>
  );
};
