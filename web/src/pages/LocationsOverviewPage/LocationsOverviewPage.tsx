import './style.scss';
import { useQuery, useSuspenseQuery } from '@tanstack/react-query';
import { useNavigate, useSearch } from '@tanstack/react-router';
import api from '../../shared/api/api';
import { GatewaySetupModal } from '../../shared/components/modals/GatewaySetupModal/GatewaySetupModal';
import { OverviewPeriodSelect } from '../../shared/components/OverviewPeriodSelect/OverviewPeriodSelect';
import { Page } from '../../shared/components/Page/Page';
import { SizedBox } from '../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../shared/defguard-ui/types';
import { isPresent } from '../../shared/defguard-ui/utils/isPresent';
import { getLocationsQueryOptions } from '../../shared/query';
import {
  LocationOverviewCard,
  OverviewCard,
} from './components/LocationOverviewCard/LocationOverviewCard';

export const LocationsOverviewPage = () => {
  const navigate = useNavigate();
  const { data: locations } = useSuspenseQuery(getLocationsQueryOptions);
  const { period } = useSearch({ from: '/_authorized/vpn-overview/' });

  const { data: allStats } = useQuery({
    queryFn: () => api.location.getLocationsSummary(period),
    queryKey: [
      'network',
      'stats',
      'all',
      {
        from: period,
      },
    ],
    select: (resp) => resp.data,
    refetchInterval: 30_000,
  });

  return (
    <>
      <Page title="VPN Overview" id="locations-overview-page">
        <SizedBox height={ThemeSpacing.Xl3} />
        <div className="top">
          <p>Dashboard</p>
          <div className="right">
            <OverviewPeriodSelect
              onChange={(value) => {
                navigate({
                  from: '/vpn-overview',
                  search: {
                    period: value,
                  },
                });
              }}
              period={period}
            />
          </div>
        </div>
        <SizedBox height={ThemeSpacing.Xl2} />
        <ul>
          {isPresent(allStats) && (
            <li>
              <OverviewCard statsPeriod={period} data={allStats} expanded={true}>
                <div className="summary-top">
                  <p>All locations summary</p>
                </div>
              </OverviewCard>
            </li>
          )}
          {locations.map((location) => (
            <li key={location.id}>
              <LocationOverviewCard location={location} statsPeriod={period} showTop />
            </li>
          ))}
        </ul>
      </Page>
      <GatewaySetupModal />
    </>
  );
};
