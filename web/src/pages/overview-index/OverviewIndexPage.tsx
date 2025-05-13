import './style.scss';

import { useQuery } from '@tanstack/react-query';
import { range } from 'lodash-es';
import { useMemo } from 'react';
import Skeleton from 'react-loading-skeleton';

import { ExpandableSection } from '../../shared/components/Layout/ExpandableSection/ExpandableSection';
import { PageContainer } from '../../shared/components/Layout/PageContainer/PageContainer';
import { AllNetworksGatewaysStatus } from '../../shared/components/network/GatewaysStatus/AllNetworksGatewaysStatus/AllNetworksGatewaysStatus';
import { NetworkGatewaysStatus } from '../../shared/components/network/GatewaysStatus/NetworkGatewaysStatus/NetworkGatewaysStatus';
import { Button } from '../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../shared/defguard-ui/components/Layout/Button/types';
import { NoData } from '../../shared/defguard-ui/components/Layout/NoData/NoData';
import { isPresent } from '../../shared/defguard-ui/utils/isPresent';
import useApi from '../../shared/hooks/useApi';
import { useToaster } from '../../shared/hooks/useToaster';
import { Network } from '../../shared/types';
import { getNetworkStatsFilterValue } from '../overview/helpers/stats';
import { useOverviewStore } from '../overview/hooks/store/useOverviewStore';
import { OverviewStats } from '../overview/OverviewStats/OverviewStats';
import { OverviewStatsFilterSelect } from '../overview/OverviewStatsFilterSelect/OverviewStatsFilterSelect';

export const OverviewIndexPage = () => {
  const {
    network: { getNetworks },
  } = useApi();

  const { data, isLoading } = useQuery({
    queryKey: ['network'],
    queryFn: getNetworks,
  });

  return (
    <PageContainer id="overview-index">
      <div className="page-limited-content">
        <header>
          <h1>All locations overview</h1>
          <div className="controls">
            <OverviewStatsFilterSelect />
          </div>
        </header>
        <ExpandableSection
          id="all-networks-summary"
          textAs="h2"
          text="All locations summary"
        >
          <AllNetworksGatewaysStatus />
          <SummaryStats />
        </ExpandableSection>
        {!data &&
          isLoading &&
          range(6).map((skeletonIndex) => <NetworkSectionSkeleton key={skeletonIndex} />)}
        {isPresent(data) &&
          !isLoading &&
          data.length > 0 &&
          data.map((network) => <NetworkSection network={network} key={network.id} />)}
        {isPresent(data) && data.length === 0 && !isLoading && (
          <NoData messagePosition="center" customMessage="No networks found" />
        )}
      </div>
    </PageContainer>
  );
};

const NetworkSectionSkeleton = () => {
  return (
    <div className="network-section-skeleton">
      <Skeleton />
      <Skeleton />
      <StatsSkeleton />
    </div>
  );
};

type NetworkSectionProps = {
  network: Network;
};

const NetworkSection = ({ network }: NetworkSectionProps) => {
  const toaster = useToaster();
  const statsFilter = useOverviewStore((s) => s.statsFilter);

  const from = useMemo(() => getNetworkStatsFilterValue(statsFilter), [statsFilter]);

  const {
    network: { getNetworkStats },
  } = useApi();

  const { data } = useQuery({
    queryFn: () => getNetworkStats({ id: network.id, from }),
    queryKey: ['network', network.id, 'stats', from],
    refetchInterval: 60 * 1000,
    placeholderData: (perv) => perv,
  });

  return (
    <ExpandableSection
      className="network-section"
      textAs="h2"
      text={network.name}
      key={network.id}
    >
      <div className="top-track">
        <NetworkGatewaysStatus networkId={network.id} />
        <Button
          size={ButtonSize.SMALL}
          styleVariant={ButtonStyleVariant.LINK}
          text="See Location Details"
          onClick={() => {
            toaster.warning('TODO: Need to refactor routing for this to work.');
          }}
        />
      </div>
      {!data && <StatsSkeleton />}
      {data && <OverviewStats networkStats={data} />}
    </ExpandableSection>
  );
};

const SummaryStats = () => {
  const statsFilter = useOverviewStore((s) => s.statsFilter);

  const from = useMemo(() => getNetworkStatsFilterValue(statsFilter), [statsFilter]);
  const {
    network: { getAllNetworksStats },
  } = useApi();
  const { data, isLoading } = useQuery({
    queryKey: ['network', 'stats', from],
    queryFn: () => getAllNetworksStats({ from }),
    refetchInterval: 60 * 1000,
    placeholderData: (perv) => perv,
  });
  return (
    <>
      {!data && isLoading && <NetworkSectionSkeleton />}
      {data && !isLoading && <OverviewStats networkStats={data} />}
    </>
  );
};

const StatsSkeleton = () => {
  return (
    <div className="network-stats-skeleton">
      <Skeleton />
      <Skeleton />
    </div>
  );
};
