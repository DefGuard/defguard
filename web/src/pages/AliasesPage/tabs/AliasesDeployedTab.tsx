import { useQuery, useSuspenseQuery } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import { useMemo, useState } from 'react';
import { m } from '../../../paraglide/messages';
import { AclStatus } from '../../../shared/api/types';
import { TableSkeleton } from '../../../shared/components/skeleton/TableSkeleton/TableSkeleton';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import type { ButtonProps } from '../../../shared/defguard-ui/components/Button/types';
import { EmptyStateFlexible } from '../../../shared/defguard-ui/components/EmptyStateFlexible/EmptyStateFlexible';
import { Search } from '../../../shared/defguard-ui/components/Search/Search';
import { TableTop } from '../../../shared/defguard-ui/components/table/TableTop/TableTop';
import { isPresent } from '../../../shared/defguard-ui/utils/isPresent';
import {
  getAliasesQueryOptions,
  getLicenseInfoQueryOptions,
  getRulesQueryOptions,
} from '../../../shared/query';
import { canUseBusinessFeature, licenseActionCheck } from '../../../shared/utils/license';
import { AliasTable } from '../AliasTable';

export const AliasesDeployedTab = () => {
  const { data: aliases } = useSuspenseQuery({
    ...getAliasesQueryOptions,
    select: (resp) => resp.data.filter((alias) => alias.state === AclStatus.Applied),
  });
  const isEmpty = aliases.length === 0;
  const navigate = useNavigate();
  const [search, setSearch] = useState('');
  const { data: licenseInfo, isFetching: licenseFetching } = useQuery(
    getLicenseInfoQueryOptions,
  );
  const {
    data: rules,
    isLoading: rulesLoading,
    isFetching: rulesFetching,
  } = useQuery(getRulesQueryOptions);
  const rulesReady = !rulesLoading && !rulesFetching && isPresent(rules);

  const addButtonProps = useMemo(
    (): ButtonProps => ({
      text: 'Add new alias',
      iconLeft: 'add-alias',
      variant: 'primary',
      testId: 'add-alias',
      disabled: licenseFetching,
      onClick: () => {
        if (licenseInfo === undefined) return;
        licenseActionCheck(canUseBusinessFeature(licenseInfo), () => {
          navigate({ to: '/acl/add-alias' });
        });
      },
    }),
    [navigate, licenseFetching, licenseInfo],
  );

  const distilledAliases = useMemo(() => {
    let res = aliases;
    if (search?.length) {
      res = res.filter((alias) =>
        alias.name.toLowerCase().includes(search.toLowerCase()),
      );
    }
    return res;
  }, [aliases, search]);

  const visibleEmpty = distilledAliases.length === 0;

  return (
    <>
      {isEmpty && (
        <EmptyStateFlexible
          icon="aliases"
          title={`You haven't created any aliases yet.`}
          subtitle="Click the first alias by clicking button below."
          primaryAction={addButtonProps}
        />
      )}
      {!isEmpty && (
        <>
          <TableTop text="Deployed aliases">
            <Search
              placeholder={m.controls_search()}
              initialValue={search}
              onChange={(search) => {
                setSearch(search);
              }}
            />
            <Button {...addButtonProps} />
          </TableTop>
          {!visibleEmpty && rulesReady && <AliasTable data={aliases} />}
          {!visibleEmpty && !rulesReady && <TableSkeleton />}
          {visibleEmpty && (
            <EmptyStateFlexible
              icon="search"
              title="No aliases found."
              subtitle="Try different search."
            />
          )}
        </>
      )}
    </>
  );
};
