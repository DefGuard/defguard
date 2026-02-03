import { useNavigate } from '@tanstack/react-router';
import { useMemo } from 'react';
import type { AclRule } from '../../../shared/api/types';
import type { ButtonProps } from '../../../shared/defguard-ui/components/Button/types';
import { EmptyStateFlexible } from '../../../shared/defguard-ui/components/EmptyStateFlexible/EmptyStateFlexible';
import { RulesTable } from '../RulesTable';

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
      {!isEmpty && (
        <RulesTable
          enableSearch
          title="Deployed rules"
          buttonProps={buttonProps}
          data={rules}
        />
      )}
    </>
  );
};
