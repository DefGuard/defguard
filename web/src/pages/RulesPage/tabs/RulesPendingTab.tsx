import { useMutation } from '@tanstack/react-query';
import { useMemo } from 'react';
import api from '../../../shared/api/api';
import type { AclRule } from '../../../shared/api/types';
import { TableSkeleton } from '../../../shared/components/skeleton/TableSkeleton/TableSkeleton';
import type { ButtonProps } from '../../../shared/defguard-ui/components/Button/types';
import { EmptyStateFlexible } from '../../../shared/defguard-ui/components/EmptyStateFlexible/EmptyStateFlexible';
import { isPresent } from '../../../shared/defguard-ui/utils/isPresent';
import { canUseBusinessFeature, licenseActionCheck } from '../../../shared/utils/license';
import { RulesTable } from '../RulesTable';
import { useRuleDeps } from '../useRuleDeps';

type Props = {
  rules: AclRule[];
};

export const RulesPendingTab = ({ rules }: Props) => {
  const isEmpty = rules.length === 0;

  const { mutate, isPending } = useMutation({
    mutationFn: api.acl.rule.applyRules,
    meta: {
      invalidate: ['acl'],
    },
  });

  const { aliases, groups, locations, users, devices, license, loading } = useRuleDeps();

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
        isPresent(groups) &&
        isPresent(locations) &&
        isPresent(users) &&
        isPresent(devices) &&
        isPresent(license) && (
          <RulesTable
            title="Pending rules"
            buttonProps={buttonProps}
            data={rules}
            aliases={aliases}
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
