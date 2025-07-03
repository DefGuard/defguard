import './style.scss';

import { useQuery } from '@tanstack/react-query';
import { orderBy, range } from 'lodash-es';
import { useEffect } from 'react';
import Skeleton from 'react-loading-skeleton';
import { useLocation, useNavigate } from 'react-router';

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
import { Network } from '../../shared/types';
import { OverviewStats } from '../overview/OverviewStats/OverviewStats';
import { useWizardStore } from '../wizard/hooks/useWizardStore';
import { EditLocationsSettingsButton } from './components/EditLocationsSettingsButton/EditLocationsSettingsButton';
import { useOverviewTimeSelection } from './components/hooks/useOverviewTimeSelection';
import { OverviewNetworkSelection } from './components/OverviewNetworkSelection/OverviewNetworkSelection';
import { OverviewTimeSelection } from './components/OverviewTimeSelection/OverviewTimeSelection';

export const OverviewIndexPage = () => {
  const {
    network: { getNetworks },
  } = useApi();

  const { data, isLoading, isStale } = useQuery({
    queryKey: ['network'],
    queryFn: getNetworks,
    placeholderData: (perv) => perv,
    select: (networks) =>
      orderBy(networks, (network) => network.name.toLowerCase(), ['asc']),
  });

  const resetWizard = useWizardStore((state) => state.resetState);
  const navigate = useNavigate();

  useEffect(() => {
    if (isPresent(data) && data.length === 0 && !isLoading && !isStale) {
      resetWizard();
      navigate('/admin/wizard', { replace: true });
    }
    if (isPresent(data) && data.length === 1) {
      const network = data[0];
      navigate(`/admin/overview/${network.id}`, { replace: true });
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [data, isLoading, isStale]);

  return (
    <PageContainer id="overview-index">
      <div className="page-limited-content">
        <header>
          <h1>All locations overview</h1>
          <div className="controls">
            <OverviewNetworkSelection />
            <OverviewTimeSelection />
          </div>
          <EditLocationsSettingsButton />
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
  const location = useLocation();
  const navigate = useNavigate();

  const { from } = useOverviewTimeSelection();

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
            navigate(`/admin/overview/${network.id}${location.search}`);
          }}
          icon={
            <svg
              xmlns="http://www.w3.org/2000/svg"
              width="12"
              height="12"
              viewBox="0 0 14 12"
              fill="none"
            >
              <path
                d="M7 3C5.62066 3 4.35814 3.6065 3.24751 4.80266C2.83022 5.25208 2.50531 5.71042 2.27279 6.08163C2.48464 6.41786 2.77518 6.82672 3.1454 7.23301C4.26222 8.45864 5.55909 9.08008 7 9.08008C8.40806 9.08008 9.67988 8.48573 10.7801 7.31354C11.1858 6.88137 11.5009 6.44061 11.727 6.08138C11.4945 5.71022 11.1697 5.25196 10.7525 4.80266C9.64186 3.6065 8.37934 3 7 3ZM7 1C11.8216 1 14 6.08008 14 6.08008C14 6.08008 11.8878 11.0801 7 11.0801C2.11224 11.0801 0 6.08008 0 6.08008C0 6.08008 2.17844 1 7 1Z"
                fill="#899CA8"
              />
              <path
                d="M5 6.08008C5 7.18465 5.89543 8.08008 7 8.08008C8.10457 8.08008 9 7.18465 9 6.08008C9 4.97551 8.10457 4.08008 7 4.08008C5.89543 4.08008 5 4.97551 5 6.08008Z"
                fill="#899CA8"
              />
            </svg>
          }
        />
      </div>
      {!data && <StatsSkeleton />}
      {data && <OverviewStats networkStats={data} />}
    </ExpandableSection>
  );
};

const SummaryStats = () => {
  const { from } = useOverviewTimeSelection();
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
