import { useSuspenseQuery } from '@tanstack/react-query';
import { Page } from '../../shared/components/Page/Page';
import { getLocationsQueryOptions } from '../../shared/query';
import { LocationsTable } from './components/LocationsTable';
import { AddLocationModal } from './modals/AddLocationModal/AddLocationModal';
import './style.scss';

export const LocationsPage = () => {
  const { data: locations } = useSuspenseQuery(getLocationsQueryOptions);

  return (
    <>
      <Page title="Locations" id="locations-page">
        <LocationsTable locations={locations} />
      </Page>
      <AddLocationModal />
    </>
  );
};
