import { Suspense } from 'react';
import { Page } from '../../shared/components/Page/Page';
import { LocationsTable } from './components/LocationsTable';
import { AddLocationModal } from './modals/AddLocationModal/AddLocationModal';
import './style.scss';
import { TableSkeleton } from '../../shared/components/skeleton/TableSkeleton/TableSkeleton';
import { TablePageLayout } from '../../shared/layout/TablePageLayout/TablePageLayout';

export const LocationsPage = () => {
  return (
    <>
      <Page title="Locations" id="locations-page">
        <TablePageLayout>
          <Suspense fallback={<TableSkeleton />}>
            <LocationsTable />
          </Suspense>
        </TablePageLayout>
      </Page>
      <AddLocationModal />
    </>
  );
};
