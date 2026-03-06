import { useMutation, useQuery, useSuspenseQuery } from '@tanstack/react-query';
import { useMemo } from 'react';
import api from '../../../../shared/api/api';
import { AclStatus } from '../../../../shared/api/types';
import { TableSkeleton } from '../../../../shared/components/skeleton/TableSkeleton/TableSkeleton';
import type { ButtonProps } from '../../../../shared/defguard-ui/components/Button/types';
import { EmptyStateFlexible } from '../../../../shared/defguard-ui/components/EmptyStateFlexible/EmptyStateFlexible';
import { isPresent } from '../../../../shared/defguard-ui/utils/isPresent';
import {
  getDestinationsQueryOptions,
  getRulesQueryOptions,
} from '../../../../shared/query';
import { DestinationsTable } from '../../components/DestinationsTable';

export const DestinationPendingTab = () => {
  const { data: destinations } = useSuspenseQuery({
    ...getDestinationsQueryOptions,
    select: (resp) =>
      resp.data.filter((destination) => destination.state !== AclStatus.Applied),
  });
  const {
    data: rules,
    isLoading: rulesLoading,
    isFetching: rulesFetching,
  } = useQuery(getRulesQueryOptions);
  const rulesReady = !rulesLoading && !rulesFetching && isPresent(rules);

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

      {destinations.length > 0 &&
        (rulesReady ? (
          <DestinationsTable
            destinations={destinations}
            primaryProps={deployPending}
            title="Pending destinations"
            disableBlockedModal
          />
        ) : (
          <TableSkeleton />
        ))}
    </>
  );
};
