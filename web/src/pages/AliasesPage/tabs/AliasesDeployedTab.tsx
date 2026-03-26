import { useSuspenseQuery } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import { useMemo, useState } from 'react';
import { m } from '../../../paraglide/messages';
import { AclStatus } from '../../../shared/api/types';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import type { ButtonProps } from '../../../shared/defguard-ui/components/Button/types';
import { EmptyStateFlexible } from '../../../shared/defguard-ui/components/EmptyStateFlexible/EmptyStateFlexible';
import { Search } from '../../../shared/defguard-ui/components/Search/Search';
import { TableTop } from '../../../shared/defguard-ui/components/table/TableTop/TableTop';
import { getAliasesQueryOptions, getRulesQueryOptions } from '../../../shared/query';
import { canUseBusinessFeature, licenseActionCheck } from '../../../shared/utils/license';
import { DeletionBlockedModal } from '../../Acl/components/DeletionBlockedModal/DeletionBlockedModal';
import { useRuleDeps } from '../../RulesPage/useRuleDeps';
import { AliasTable } from '../AliasTable';

export const AliasesDeployedTab = () => {
  const { data: aliases } = useSuspenseQuery({
    ...getAliasesQueryOptions,
    select: (resp) => resp.data.filter((alias) => alias.state === AclStatus.Applied),
  });
  const isEmpty = aliases.length === 0;
  const navigate = useNavigate();
  const [search, setSearch] = useState('');
  const { license, loading } = useRuleDeps();
  const { data: rules } = useSuspenseQuery(getRulesQueryOptions);
  const rulesByAliasId = useMemo(() => {
    const map: Record<number, string[]> = {};

    rules.forEach((rule) => {
      rule.aliases.forEach((aliasId) => {
        if (!map[aliasId]) {
          map[aliasId] = [];
        }

        map[aliasId].push(rule.name);
      });
    });

    return map;
  }, [rules]);

  const addButtonProps = useMemo(
    (): ButtonProps => ({
      text: m.acl_aliases_button_create(),
      iconLeft: 'add-alias',
      variant: 'primary',
      testId: 'add-alias',
      disabled: loading,
      onClick: () => {
        if (license === undefined) return;
        licenseActionCheck(canUseBusinessFeature(license), () => {
          navigate({ to: '/acl/add-alias' });
        });
      },
    }),
    [navigate, loading, license],
  );

  const filteredAliases = useMemo(() => {
    if (!search.length) {
      return aliases;
    }

    const normalizedSearch = search.toLowerCase();

    return aliases.filter((alias) => {
      if (alias.name.toLowerCase().includes(normalizedSearch)) {
        return true;
      }

      const aliasId = alias.parent_id ?? alias.id;
      const ruleNames = rulesByAliasId[aliasId] ?? [];

      return ruleNames.some((ruleName) =>
        ruleName.toLowerCase().includes(normalizedSearch),
      );
    });
  }, [aliases, rulesByAliasId, search]);

  const visibleEmpty = filteredAliases.length === 0;

  return (
    <>
      {isEmpty && (
        <EmptyStateFlexible
          icon="aliases"
          title={m.acl_aliases_empty_deployed_title()}
          subtitle={m.acl_aliases_empty_deployed_subtitle()}
          primaryAction={addButtonProps}
        />
      )}
      {!isEmpty && (
        <>
          <TableTop text={m.acl_aliases_table_title_deployed()}>
            <Search
              placeholder={m.controls_search()}
              initialValue={search}
              onChange={(search) => {
                setSearch(search);
              }}
            />
            <Button {...addButtonProps} />
          </TableTop>
          {!visibleEmpty && <AliasTable data={filteredAliases} rules={rules} />}
          {visibleEmpty && (
            <EmptyStateFlexible
              icon="search"
              title={m.acl_aliases_search_empty_title()}
              subtitle={m.acl_aliases_search_empty_subtitle()}
            />
          )}
        </>
      )}
      <DeletionBlockedModal />
    </>
  );
};
