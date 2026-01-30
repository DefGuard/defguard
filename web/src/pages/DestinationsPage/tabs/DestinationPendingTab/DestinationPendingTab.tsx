import { useMemo } from 'react';
import type { AclDestination } from '../../../../shared/api/types';
import type { ButtonProps } from '../../../../shared/defguard-ui/components/Button/types';
import { EmptyStateFlexible } from '../../../../shared/defguard-ui/components/EmptyStateFlexible/EmptyStateFlexible';
import { DestinationsTable } from '../../components/DestinationsTable';

type Props = {
  destinations: AclDestination[];
};

export const DestinationPendingTab = ({ destinations }: Props) => {
  const deployPending = useMemo(
    (): ButtonProps => ({
      text: 'Deploy all pending',
      iconLeft: 'deploy',
      onClick: () => {},
      disabled: true,
    }),
    [],
  );
  return (
    <>
      {destinations.length === 0 && (
        <EmptyStateFlexible
          icon="gateway"
          title="You don't have any pending items."
          subtitle="They will appear here once you add or modify one."
        />
      )}

      {destinations.length > 0 && (
        <DestinationsTable
          destinations={destinations}
          primaryProps={deployPending}
          title="Pending destinations"
        />
      )}
    </>
  );
};
