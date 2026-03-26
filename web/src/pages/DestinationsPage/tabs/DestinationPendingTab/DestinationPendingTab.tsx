import { useMutation, useSuspenseQuery } from '@tanstack/react-query';
import { useMemo } from 'react';
import { m } from '../../../../paraglide/messages';
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
      text: m.acl_destinations_button_deploy_all_pending({ count: destinations.length }),
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
          title={m.acl_destinations_empty_pending_title()}
          subtitle={m.acl_destinations_empty_pending_subtitle()}
        />
      )}

      {destinations.length > 0 && (
        <DestinationsTable
          destinations={destinations}
          rules={rules}
          primaryProps={deployPending}
          title={m.acl_destinations_table_title_pending()}
          disableBlockedModal
        />
      )}
    </>
  );
};
