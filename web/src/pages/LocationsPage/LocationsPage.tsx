import { useSuspenseQuery } from '@tanstack/react-query';
import { Page } from '../../shared/components/Page/Page';
import { LocationsTable } from './components/LocationsTable';
import './style.scss';
import { GatewaySetupModal } from '../../shared/components/modals/GatewaySetupModal/GatewaySetupModal';
import { getLocationsQueryOptions } from '../../shared/query';

export const LocationsPage = () => {
  const { data: locations } = useSuspenseQuery(getLocationsQueryOptions);
  return (
    <>
      <Page title="Locations" id="locations-page">
        <LocationsTable locations={locations} />
      </Page>
      <GatewaySetupModal />
    </>
  );
};
