import './style.scss';
import { useSuspenseQuery } from '@tanstack/react-query';
import { Page } from '../../shared/components/Page/Page';
import { SizedBox } from '../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../shared/defguard-ui/types';
import { getLocationsQueryOptions } from '../../shared/query';
import { LocationOverviewCard } from './components/LocationOverviewCard/LocationOverviewCard';

export const LocationsOverviewPage = () => {
  const { data: locations } = useSuspenseQuery(getLocationsQueryOptions);

  return (
    <Page title="VPN Overview" id="locations-overview-page">
      <SizedBox height={ThemeSpacing.Xl3} />
      <div className="top">
        <p>Dashboard</p>
      </div>
      <SizedBox height={ThemeSpacing.Xl2} />
      <ul>
        {locations.map((location) => (
          <li key={location.id}>
            <LocationOverviewCard location={location} showTop />
          </li>
        ))}
      </ul>
    </Page>
  );
};
