import { useQuery } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import { useMemo } from 'react';
import type { AclDestination } from '../../../../shared/api/types';
import type { ButtonProps } from '../../../../shared/defguard-ui/components/Button/types';
import { EmptyStateFlexible } from '../../../../shared/defguard-ui/components/EmptyStateFlexible/EmptyStateFlexible';
import { getLicenseInfoQueryOptions } from '../../../../shared/query';
import {
  canUseBusinessFeature,
  licenseActionCheck,
} from '../../../../shared/utils/license';
import { DestinationsTable } from '../../components/DestinationsTable';

type Props = {
  destinations: AclDestination[];
};

export const DestinationDeployedTab = ({ destinations }: Props) => {
  const navigate = useNavigate();

  const { data: licenseInfo, isFetching } = useQuery(getLicenseInfoQueryOptions);

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
        <DestinationsTable
          title="Deployed destinations"
          destinations={destinations}
          primaryProps={addButtonProps}
          search
        />
      )}
    </>
  );
};
