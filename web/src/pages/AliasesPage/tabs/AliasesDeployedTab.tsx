import { useQuery } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import { useMemo, useState } from 'react';
import { m } from '../../../paraglide/messages';
import type { AclAlias } from '../../../shared/api/types';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import type { ButtonProps } from '../../../shared/defguard-ui/components/Button/types';
import { EmptyStateFlexible } from '../../../shared/defguard-ui/components/EmptyStateFlexible/EmptyStateFlexible';
import { Search } from '../../../shared/defguard-ui/components/Search/Search';
import { TableTop } from '../../../shared/defguard-ui/components/table/TableTop/TableTop';
import { getLicenseInfoQueryOptions } from '../../../shared/query';
import { canUseBusinessFeature, licenseActionCheck } from '../../../shared/utils/license';
import { AliasTable } from '../AliasTable';

type Props = {
  aliases: AclAlias[];
};

export const AliasesDeployedTab = ({ aliases }: Props) => {
  const isEmpty = aliases.length === 0;
  const navigate = useNavigate();
  const [search, setSearch] = useState('');
  const { data: licenseInfo, isFetching: licenseFetching } = useQuery(
    getLicenseInfoQueryOptions,
  );

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
          {!visibleEmpty && <AliasTable data={aliases} />}
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
