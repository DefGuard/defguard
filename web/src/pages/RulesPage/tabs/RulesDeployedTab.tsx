import { useSuspenseQuery } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import { useMemo } from 'react';
import { m } from '../../../paraglide/messages';
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
      text: m.acl_rules_button_create(),
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
          title={m.acl_rules_empty_deployed_title()}
          subtitle={m.acl_rules_empty_deployed_subtitle()}
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
            title={m.acl_rules_table_title_deployed()}
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
