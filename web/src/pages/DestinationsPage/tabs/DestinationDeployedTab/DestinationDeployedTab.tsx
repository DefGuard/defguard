import { useQuery, useSuspenseQuery } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import { useMemo } from 'react';
import { AclStatus } from '../../../../shared/api/types';
import { TableSkeleton } from '../../../../shared/components/skeleton/TableSkeleton/TableSkeleton';
import type { ButtonProps } from '../../../../shared/defguard-ui/components/Button/types';
import { EmptyStateFlexible } from '../../../../shared/defguard-ui/components/EmptyStateFlexible/EmptyStateFlexible';
import { isPresent } from '../../../../shared/defguard-ui/utils/isPresent';
import {
  getDestinationsQueryOptions,
  getLicenseInfoQueryOptions,
  getRulesQueryOptions,
} from '../../../../shared/query';
import {
  canUseBusinessFeature,
  licenseActionCheck,
} from '../../../../shared/utils/license';
import { DestinationsTable } from '../../components/DestinationsTable';

export const DestinationDeployedTab = () => {
  const { data: destinations } = useSuspenseQuery({
    ...getDestinationsQueryOptions,
    select: (resp) =>
      resp.data.filter((destination) => destination.state === AclStatus.Applied),
  });
  const navigate = useNavigate();

  const { data: licenseInfo, isFetching } = useQuery(getLicenseInfoQueryOptions);
  const {
    data: rules,
    isLoading: rulesLoading,
    isFetching: rulesFetching,
  } = useQuery(getRulesQueryOptions);
  const rulesReady = !rulesLoading && !rulesFetching && isPresent(rules);

  const addButtonProps = useMemo(
    (): ButtonProps => ({
      text: 'Add new destination',
      variant: 'primary',
      iconLeft: 'add-location',
      disabled: isFetching,
      onClick: () => {
        if (licenseInfo === undefined) return;
        licenseActionCheck(canUseBusinessFeature(licenseInfo), () => {
          navigate({
            to: '/acl/add-destination',
          });
        });
      },
    }),
    [navigate, isFetching, licenseInfo],
  );

  return (
    <>
      {destinations.length === 0 && (
        <EmptyStateFlexible
          icon="gateway"
          title="You haven't created any destinations yet."
          subtitle="Click the first destination by clicking button below."
          primaryAction={addButtonProps}
        />
      )}
      {destinations.length > 0 && (
        <>
          {rulesReady && (
            <DestinationsTable
              title="Deployed destinations"
              destinations={destinations}
              primaryProps={addButtonProps}
              search
            />
          )}
          {!rulesReady && <TableSkeleton />}
        </>
      )}
    </>
  );
};
