import { useMutation } from '@tanstack/react-query';
import { useMemo } from 'react';
import api from '../../../../shared/api/api';
import type { AclDestination } from '../../../../shared/api/types';
import type { ButtonProps } from '../../../../shared/defguard-ui/components/Button/types';
import { EmptyStateFlexible } from '../../../../shared/defguard-ui/components/EmptyStateFlexible/EmptyStateFlexible';
import { DestinationsTable } from '../../components/DestinationsTable';

type Props = {
  destinations: AclDestination[];
};

export const DestinationPendingTab = ({ destinations }: Props) => {
  const { mutate, isPending } = useMutation({
    mutationFn: api.acl.destination.applyDestinations,
    meta: {
      invalidate: ['acl'],
    },
  });

  const deployPending = useMemo(
    (): ButtonProps => ({
      text: `Deploy all pending (${destinations.length})`,
      iconLeft: 'deploy',
      loading: isPending,
      onClick: () => {
        mutate(destinations.map((destination) => destination.id));
      },
    }),
    [isPending, mutate, destinations],
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
