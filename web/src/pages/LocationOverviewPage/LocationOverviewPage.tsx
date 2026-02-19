import { useQuery, useSuspenseQuery } from '@tanstack/react-query';
import { Page } from '../../shared/components/Page/Page';
import './style.scss';
import { useNavigate, useParams, useSearch } from '@tanstack/react-router';
import { useMemo, useState } from 'react';
import api from '../../shared/api/api';
import { GatewaysStatusBadge } from '../../shared/components/GatewaysStatusBadge/GatewaysStatusBadge';
import { OverviewPeriodSelect } from '../../shared/components/OverviewPeriodSelect/OverviewPeriodSelect';
import { SizedBox } from '../../shared/defguard-ui/components/SizedBox/SizedBox';
import { Tabs } from '../../shared/defguard-ui/components/Tabs/Tabs';
import type { TabsItem } from '../../shared/defguard-ui/components/Tabs/types';
import { ThemeSpacing } from '../../shared/defguard-ui/types';
import { isPresent } from '../../shared/defguard-ui/utils/isPresent';
import { getLocationQueryOptions } from '../../shared/query';
import { OverviewCard } from '../LocationsOverviewPage/components/LocationOverviewCard/LocationOverviewCard';
import { LocationOverviewNetworkDevicesTable } from './LocationOverviewNetworkDevicesTable';
import { LocationOverviewUsersTable } from './LocationOverviewUsersTable';

export const LocationOverviewPage = () => {
  const search = useSearch({ from: '/_authorized/_default/vpn-overview/$locationId' });
  const navigate = useNavigate({ from: '/vpn-overview/$locationId' });

  const { locationId } = useParams({
    from: '/_authorized/_default/vpn-overview/$locationId',
  });

  const { data: location } = useSuspenseQuery(
    getLocationQueryOptions(parseInt(locationId, 10)),
  );

  const { data: gateways } = useQuery({
    queryFn: () => api.location.getLocationGatewaysStatus(Number(locationId)),
    queryKey: ['network', locationId, 'gateway'],
    select: (resp) => resp.data,
    refetchInterval: 60_000,
  });

  const { data: locationStats } = useQuery({
    queryFn: () =>
      api.location.getLocationStats({
        id: Number(locationId),
        from: search.period,
      }),
    queryKey: ['network', Number(locationId), 'stats', search.period],
    select: (resp) => resp.data,
    refetchInterval: 30_000,
  });

  return (
    <Page title="VPN Overview" id="location-overview-page">
      <SizedBox height={ThemeSpacing.Xl3} />
      <div className="info">
        <div className="top">
          <div className="left">
            <p className="subtitle">{location.name}</p>
            {isPresent(gateways) && <GatewaysStatusBadge data={gateways ?? []} />}
          </div>
          <div className="right">
            <OverviewPeriodSelect
              period={search.period}
              onChange={(value) => {
                navigate({
                  search: {
                    period: value,
                  },
                });
              }}
            />
          </div>
        </div>
        {isPresent(locationStats) && (
          <OverviewCard expanded statsPeriod={search.period} data={locationStats} />
        )}
      </div>
      <SizedBox height={ThemeSpacing.Xl4} />
      <DevicesSection />
    </Page>
  );
};

const DevicesSection = () => {
  const [selected, setSelected] = useState<'users' | 'devices'>('users');

  const tabItems = useMemo(
    (): TabsItem[] => [
      {
        title: 'Users',
        active: selected === 'users',
        onClick: () => setSelected('users'),
      },
      {
        title: 'Network devices',
        active: selected === 'devices',
        onClick: () => setSelected('devices'),
      },
    ],
    [selected],
  );
  return (
    <>
      <div className="table-selection">
        <p className="table-title">
          {selected === 'users' && "Connected users' devices"}
          {selected === 'devices' && 'Connected network devices'}
        </p>
        <SizedBox height={ThemeSpacing.Lg} />
        <Tabs items={tabItems} />
        <SizedBox height={ThemeSpacing.Lg} />
      </div>
      {selected === 'users' && <LocationOverviewUsersTable />}
      {selected === 'devices' && <LocationOverviewNetworkDevicesTable />}
    </>
  );
};
