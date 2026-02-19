import { Suspense } from 'react';
import { TableSkeleton } from '../../../shared/components/skeleton/TableSkeleton/TableSkeleton';

export const GatewaysTab = () => {
  return (
    <Suspense fallback={<TableSkeleton />}>
      <div>Gateways table</div>
    </Suspense>
  );
};
