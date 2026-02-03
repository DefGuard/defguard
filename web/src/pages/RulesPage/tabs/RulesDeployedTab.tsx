import { useNavigate } from '@tanstack/react-router';
import { useMemo } from 'react';
import type { AclRule } from '../../../shared/api/types';
import { TableSkeleton } from '../../../shared/components/skeleton/TableSkeleton/TableSkeleton';
import type { ButtonProps } from '../../../shared/defguard-ui/components/Button/types';
import { EmptyStateFlexible } from '../../../shared/defguard-ui/components/EmptyStateFlexible/EmptyStateFlexible';
import { isPresent } from '../../../shared/defguard-ui/utils/isPresent';
import { RulesTable } from '../RulesTable';
import { useRuleDeps } from '../useRuleDeps';

type Props = {
  rules: AclRule[];
};

export const RulesDeployedTab = ({ rules }: Props) => {
  const isEmpty = rules.length === 0;

  const navigate = useNavigate();

  const buttonProps = useMemo(
    (): ButtonProps => ({
      variant: 'primary',
      text: 'Create new rule',
      iconLeft: 'add-rule',
      onClick: () => {
        navigate({ to: '/acl/add-rule' });
      },
    }),
    [navigate],
  );

  const { aliases, groups, locations, users, devices, loading } = useRuleDeps();

  return (
    <>
      {isEmpty && (
        <EmptyStateFlexible
          icon="rules"
          title={`You don't have any firewall rules yet.`}
          subtitle={`Click the first rule by clicking button bellow.`}
          primaryAction={buttonProps}
        />
      )}
      {!isEmpty && loading && <TableSkeleton />}
      {!isEmpty &&
        isPresent(aliases) &&
        isPresent(groups) &&
        isPresent(locations) &&
        isPresent(users) &&
        isPresent(devices) && (
          <RulesTable
            title="Deployed rules"
            buttonProps={buttonProps}
            data={rules}
            aliases={aliases}
            groups={groups}
            devices={devices}
            users={users}
            locations={locations}
            enableSearch
          />
        )}
    </>
  );
};
