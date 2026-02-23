import { Suspense } from 'react';
import { TableSkeleton } from '../../../shared/components/skeleton/TableSkeleton/TableSkeleton';
import { LocationsTable } from '../components/LocationsTable';
import { AddLocationModal } from '../modals/AddLocationModal/AddLocationModal';
import { DeleteLocationModal } from '../modals/DeleteLocationModal/DeleteLocationModal';

export const LocationsTab = () => {
  return (
    <>
      <Suspense fallback={<TableSkeleton />}>
        <LocationsTable />
      </Suspense>
      <AddLocationModal />
      <DeleteLocationModal />
    </>
  );
};
