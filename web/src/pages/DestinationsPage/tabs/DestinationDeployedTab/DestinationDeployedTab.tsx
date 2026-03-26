import { useSuspenseQuery } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import { useMemo } from 'react';
import { m } from '../../../../paraglide/messages';
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
import { DeletionBlockedModal } from '../../../Acl/components/DeletionBlockedModal/DeletionBlockedModal';
import { useRuleDeps } from '../../../RulesPage/useRuleDeps';
import { DestinationsTable } from '../../components/DestinationsTable';

export const DestinationDeployedTab = () => {
  const { data: destinations } = useSuspenseQuery({
    ...getDestinationsQueryOptions,
    select: (resp) =>
      resp.data.filter((destination) => destination.state === AclStatus.Applied),
  });
  const navigate = useNavigate();

  const { data: rules } = useSuspenseQuery(getRulesQueryOptions);
  const { license, loading } = useRuleDeps();

  const addButtonProps = useMemo(
    (): ButtonProps => ({
      text: m.acl_destinations_button_create(),
      variant: 'primary',
      iconLeft: 'add-location',
      disabled: loading,
      onClick: () => {
        if (license === undefined) return;
        licenseActionCheck(canUseBusinessFeature(license), () => {
          navigate({
            to: '/acl/add-destination',
          });
        });
      },
    }),
    [navigate, loading, license],
  );

  return (
    <>
      {destinations.length === 0 && (
        <EmptyStateFlexible
          icon="gateway"
          title={m.acl_destinations_empty_deployed_title()}
          subtitle={m.acl_destinations_empty_deployed_subtitle()}
          primaryAction={addButtonProps}
        />
      )}
      {destinations.length > 0 && (
        <DestinationsTable
          title={m.acl_destinations_table_title_deployed()}
          destinations={destinations}
          rules={rules}
          primaryProps={addButtonProps}
          search
        />
      )}
      <DeletionBlockedModal />
    </>
  );
};
