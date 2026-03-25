import { useMutation, useSuspenseQuery } from '@tanstack/react-query';
import { useMemo } from 'react';
import { AclListTab } from '../../../../shared/aclTabs';
import api from '../../../../shared/api/api';
import { AclStatus } from '../../../../shared/api/types';
import type { ButtonProps } from '../../../../shared/defguard-ui/components/Button/types';
import { EmptyStateFlexible } from '../../../../shared/defguard-ui/components/EmptyStateFlexible/EmptyStateFlexible';
import {
  getDestinationsQueryOptions,
  getRulesQueryOptions,
} from '../../../../shared/query';
import {
  canUseBusinessFeature,
  licenseActionCheck,
} from '../../../../shared/utils/license';
import { useRuleDeps } from '../../../RulesPage/useRuleDeps';
import { DestinationsTable } from '../../components/DestinationsTable';

export const DestinationPendingTab = () => {
  const { data: destinations } = useSuspenseQuery({
    ...getDestinationsQueryOptions,
    select: (resp) =>
      resp.data.filter((destination) => destination.state !== AclStatus.Applied),
  });
  const { data: rules } = useSuspenseQuery(getRulesQueryOptions);
  const { license, loading } = useRuleDeps();

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
      disabled: loading,
      onClick: () => {
        if (license === undefined) return;
        licenseActionCheck(canUseBusinessFeature(license), () => {
          mutate(destinations.map((destination) => destination.id));
        });
      },
    }),
    [isPending, mutate, destinations, license, loading],
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
          rules={rules}
          tab={AclListTab.Pending}
          primaryProps={deployPending}
          title="Pending destinations"
          disableBlockedModal
        />
      )}
    </>
  );
};
