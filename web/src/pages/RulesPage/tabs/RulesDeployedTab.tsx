import { useMemo, useState } from 'react';
import type { AclRule } from '../../../shared/api/types';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import type { ButtonProps } from '../../../shared/defguard-ui/components/Button/types';
import { EmptyStateFlexible } from '../../../shared/defguard-ui/components/EmptyStateFlexible/EmptyStateFlexible';
import { Search } from '../../../shared/defguard-ui/components/Search/Search';
import { TableTop } from '../../../shared/defguard-ui/components/table/TableTop/TableTop';

type Props = {
  rules: AclRule[];
};

export const RulesDeployedTab = ({ rules }: Props) => {
  const isEmpty = rules.length === 0;

  const [search, setSearch] = useState('');

  const addRuleProps = useMemo(
    (): ButtonProps => ({
      variant: 'primary',
      text: 'Create new rule',
      iconLeft: 'add-rule',
    }),
    [],
  );

  const visibleRules = useMemo(() => {
    if (search?.length) {
      return rules.filter((rule) =>
        rule.name.toLowerCase().includes(search.toLowerCase()),
      );
    }
    return rules;
  }, [rules, search]);

  return (
    <>
      {isEmpty && (
        <EmptyStateFlexible
          icon="rules"
          title={`You don't have any firewall rules yet.`}
          subtitle={`Click the first rule by clicking button bellow.`}
          primaryAction={addRuleProps}
        />
      )}
      {!isEmpty && (
        <>
          <TableTop text="Deployed rules">
            <Search initialValue={search} onChange={setSearch} />
            <Button {...addRuleProps} />
          </TableTop>
          {visibleRules.length === 0 && (
            <EmptyStateFlexible
              icon="search"
              title="No Rules found."
              subtitle={`There isn't any rules matching your search criteria.`}
            />
          )}
        </>
      )}
    </>
  );
};
