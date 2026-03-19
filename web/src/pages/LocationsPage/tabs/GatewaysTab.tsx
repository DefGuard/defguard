import { Suspense } from 'react';
import { TableSkeleton } from '../../../shared/components/skeleton/TableSkeleton/TableSkeleton';
import { GatewaysTable } from '../components/GatewaysTable';

export const GatewaysTab = () => {
  return (
    <Suspense fallback={<TableSkeleton />}>
      <GatewaysTable />
    </Suspense>
  );
};
