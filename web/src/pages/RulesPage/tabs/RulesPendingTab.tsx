import { useMutation, useSuspenseQuery } from '@tanstack/react-query';
import { useMemo } from 'react';
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
    select: (resp) => resp.data.filter((rule) => rule.state !== AclStatus.Applied),
  });
  const isEmpty = rules.length === 0;

  const { mutate, isPending } = useMutation({
    mutationFn: api.acl.rule.applyRules,
    meta: {
      invalidate: ['acl'],
    },
  });

  const { aliases, destinations, groups, locations, users, devices, license, loading } =
    useRuleDeps();

  const buttonProps = useMemo(
    (): ButtonProps => ({
      text: `Deploy all pending (${rules.length})`,
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
          title={`You don't have any pending rules.`}
          subtitle={`They will appear here once your create your first rule.`}
        />
      )}
      {!isEmpty && loading && <TableSkeleton />}
      {!isEmpty &&
        isPresent(aliases) &&
        isPresent(destinations) &&
        isPresent(groups) &&
        isPresent(locations) &&
        isPresent(users) &&
        isPresent(devices) &&
        license !== undefined && (
          <RulesTable
            title="Pending rules"
            buttonProps={buttonProps}
            data={rules}
            aliases={aliases}
            destinations={destinations}
            groups={groups}
            devices={devices}
            users={users}
            locations={locations}
            license={license}
          />
        )}
    </>
  );
};
