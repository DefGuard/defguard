import './style.scss';
import { useSuspenseQuery } from '@tanstack/react-query';
import { useSearch } from '@tanstack/react-router';
import { OverviewPeriodSelect } from '../../shared/components/OverviewPeriodSelect/OverviewPeriodSelect';
import { Page } from '../../shared/components/Page/Page';
import { SizedBox } from '../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../shared/defguard-ui/types';
import { getLocationsQueryOptions } from '../../shared/query';
import { LocationOverviewCard } from './components/LocationOverviewCard/LocationOverviewCard';

export const LocationsOverviewPage = () => {
  const { data: locations } = useSuspenseQuery(getLocationsQueryOptions);
  const { period } = useSearch({ from: '/_authorized/vpn-overview/' });

  return (
    <Page title="VPN Overview" id="locations-overview-page">
      <SizedBox height={ThemeSpacing.Xl3} />
      <div className="top">
        <p>Dashboard</p>
        <div className="right">
          <OverviewPeriodSelect />
        </div>
      </div>
      <SizedBox height={ThemeSpacing.Xl2} />
      <ul>
        {locations.map((location) => (
          <li key={location.id}>
            <LocationOverviewCard location={location} statsPeriod={period} showTop />
          </li>
        ))}
      </ul>
    </Page>
  );
};
