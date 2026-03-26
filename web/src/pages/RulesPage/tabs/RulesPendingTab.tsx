import { useMutation, useSuspenseQuery } from '@tanstack/react-query';
import { useMemo } from 'react';
import { m } from '../../../paraglide/messages';
import api from '../../../shared/api/api';
import { AclStatus } from '../../../shared/api/types';
import { TableSkeleton } from '../../../shared/components/skeleton/TableSkeleton/TableSkeleton';
import type { ButtonProps } from '../../../shared/defguard-ui/components/Button/types';
import { EmptyStateFlexible } from '../../../shared/defguard-ui/components/EmptyStateFlexible/EmptyStateFlexible';
import { isPresent } from '../../../shared/defguard-ui/utils/isPresent';
import { getRulesQueryOptions } from '../../../shared/query';
import { canUseBusinessFeature, licenseActionCheck } from '../../../shared/utils/license';
import { RulesTable } from '../RulesTable';
import { useRuleDeps } from '../useRuleDeps';

export const RulesPendingTab = () => {
  const { data: rules } = useSuspenseQuery({
    ...getRulesQueryOptions,
    select: (rules) => rules.filter((rule) => rule.state !== AclStatus.Applied),
  });
  const isEmpty = rules.length === 0;

  const { mutate, isPending } = useMutation({
    mutationFn: api.acl.rule.applyRules,
    meta: {
      invalidate: ['acl'],
    },
  });

  const { aliases, destinations, locations, license, loading } = useRuleDeps();

  const buttonProps = useMemo(
    (): ButtonProps => ({
      text: m.acl_rules_button_deploy_all_pending({ count: rules.length }),
      iconLeft: 'deploy',
      variant: 'primary',
      loading: isPending,
      disabled: loading,
      onClick: () => {
        if (license === undefined) return;

        licenseActionCheck(canUseBusinessFeature(license), () => {
          mutate(rules.map((rule) => rule.id));
        });
      },
    }),
    [mutate, rules, license, loading, isPending],
  );

  return (
    <>
      {isEmpty && (
        <EmptyStateFlexible
          icon="rules"
          title={m.acl_rules_empty_pending_title()}
          subtitle={m.acl_rules_empty_pending_subtitle()}
        />
      )}
      {!isEmpty && loading && <TableSkeleton />}
      {!isEmpty &&
        isPresent(aliases) &&
        isPresent(destinations) &&
        isPresent(locations) &&
        license !== undefined && (
          <RulesTable
            variant="pending"
            title={m.acl_rules_table_title_pending()}
            buttonProps={buttonProps}
            data={rules}
            aliases={aliases}
            destinations={destinations}
            locations={locations}
            license={license}
          />
        )}
    </>
  );
};
