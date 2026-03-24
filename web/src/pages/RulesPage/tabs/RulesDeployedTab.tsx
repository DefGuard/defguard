import { useSuspenseQuery } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import { useMemo } from 'react';
import { AclStatus } from '../../../shared/api/types';
import { TableSkeleton } from '../../../shared/components/skeleton/TableSkeleton/TableSkeleton';
import type { ButtonProps } from '../../../shared/defguard-ui/components/Button/types';
import { EmptyStateFlexible } from '../../../shared/defguard-ui/components/EmptyStateFlexible/EmptyStateFlexible';
import { isPresent } from '../../../shared/defguard-ui/utils/isPresent';
import { getRulesQueryOptions } from '../../../shared/query';
import { canUseBusinessFeature, licenseActionCheck } from '../../../shared/utils/license';
import { RulesTable } from '../RulesTable';
import { useRuleDeps } from '../useRuleDeps';

export const RulesDeployedTab = () => {
  const { data: rules } = useSuspenseQuery({
    ...getRulesQueryOptions,
    select: (rules) => rules.filter((rule) => rule.state === AclStatus.Applied),
  });

  const isEmpty = rules.length === 0;

  const navigate = useNavigate();

  const { aliases, destinations, locations, license, loading } = useRuleDeps();

  const buttonProps = useMemo(
    (): ButtonProps => ({
      variant: 'primary',
      text: 'Create new rule',
      iconLeft: 'add-rule',
      disabled: loading,
      onClick: () => {
        if (license === undefined) return;

        licenseActionCheck(canUseBusinessFeature(license), () => {
          navigate({ to: '/acl/add-rule' });
        });
      },
    }),
    [navigate, loading, license],
  );

  return (
    <>
      {isEmpty && (
        <EmptyStateFlexible
          icon="rules"
          title={`You don't have any firewall rules yet.`}
          subtitle={`Click the first rule by clicking button below.`}
          primaryAction={buttonProps}
        />
      )}
      {!isEmpty && loading && <TableSkeleton />}
      {!isEmpty &&
        isPresent(aliases) &&
        isPresent(destinations) &&
        isPresent(locations) &&
        license !== undefined && (
          <RulesTable
            variant="deployed"
            title="Deployed rules"
            buttonProps={buttonProps}
            data={rules}
            aliases={aliases}
            destinations={destinations}
            locations={locations}
            license={license}
            enableSearch
          />
        )}
    </>
  );
};
