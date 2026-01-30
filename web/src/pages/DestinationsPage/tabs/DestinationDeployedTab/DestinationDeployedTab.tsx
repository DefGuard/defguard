import { useNavigate } from '@tanstack/react-router';
import { useMemo } from 'react';
import type { AclDestination } from '../../../../shared/api/types';
import type { ButtonProps } from '../../../../shared/defguard-ui/components/Button/types';
import { EmptyStateFlexible } from '../../../../shared/defguard-ui/components/EmptyStateFlexible/EmptyStateFlexible';
import { DestinationsTable } from '../../components/DestinationsTable';

type Props = {
  destinations: AclDestination[];
};

export const DestinationDeployedTab = ({ destinations }: Props) => {
  const navigate = useNavigate();

  const addButtonProps = useMemo(
    (): ButtonProps => ({
      text: 'Add new destination',
      variant: 'primary',
      iconLeft: 'add-location',
      onClick: () => {
        navigate({
          to: '/acl/add-destination',
        });
      },
    }),
    [navigate],
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
