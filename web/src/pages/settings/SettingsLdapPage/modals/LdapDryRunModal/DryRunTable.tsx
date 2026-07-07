import {
  createColumnHelper,
  getCoreRowModel,
  getSortedRowModel,
  useReactTable,
} from '@tanstack/react-table';
import { useMemo, useState } from 'react';
import { m } from '../../../../../paraglide/messages';
import type { LdapDryRunUser } from '../../../../../shared/api/types';
import { Badge } from '../../../../../shared/defguard-ui/components/Badge/Badge';
import { BadgeVariant } from '../../../../../shared/defguard-ui/components/Badge/types';
import { EmptyState } from '../../../../../shared/defguard-ui/components/EmptyState/EmptyState';
import { Search } from '../../../../../shared/defguard-ui/components/Search/Search';
import { SizedBox } from '../../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { TableBody } from '../../../../../shared/defguard-ui/components/table/TableBody/TableBody';
import { TableCell } from '../../../../../shared/defguard-ui/components/table/TableCell/TableCell';
import { ThemeSpacing } from '../../../../../shared/defguard-ui/types';

const columnHelper = createColumnHelper<LdapDryRunUser>();

const textCell = (info: { getValue: () => string }) => (
  <TableCell>
    <span>{info.getValue()}</span>
  </TableCell>
);

export const DryRunTable = ({ data }: { data: LdapDryRunUser[] }) => {
  const [search, setSearch] = useState('');

  const columns = useMemo(
    () => [
      columnHelper.accessor('username', {
        header: m.modal_ldap_dry_run_col_username(),
        enableSorting: true,
        sortingFn: 'text',
        size: 160,
        minSize: 160,
        cell: textCell,
      }),
      columnHelper.accessor('email', {
        header: m.modal_ldap_dry_run_col_email(),
        enableSorting: true,
        sortingFn: 'text',
        size: 260,
        minSize: 260,
        cell: textCell,
      }),
      columnHelper.accessor('action', {
        header: m.modal_ldap_dry_run_col_status(),
        enableSorting: true,
        sortingFn: 'text',
        size: 160,
        minSize: 120,
        meta: { flex: true },
        cell: (info) => {
          const added = info.getValue() === 'add';
          return (
            <TableCell flex>
              <Badge
                variant={added ? BadgeVariant.Success : BadgeVariant.Critical}
                text={
                  added
                    ? m.modal_ldap_dry_run_status_added()
                    : m.modal_ldap_dry_run_status_removed()
                }
              />
            </TableCell>
          );
        },
      }),
    ],
    [],
  );

  const filtered = useMemo(() => {
    const query = search.trim().toLowerCase();
    if (!query) return data;
    return data.filter(
      (user) =>
        user.username.toLowerCase().includes(query) ||
        user.email.toLowerCase().includes(query),
    );
  }, [data, search]);

  const table = useReactTable({
    columns,
    data: filtered,
    enableRowSelection: false,
    columnResizeMode: 'onChange',
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
  });

  if (data.length === 0) {
    return (
      <EmptyState
        icon="users"
        title={m.modal_ldap_dry_run_empty_title()}
        subtitle={m.modal_ldap_dry_run_empty_subtitle()}
      />
    );
  }

  return (
    <>
      <SizedBox height={ThemeSpacing.Xl} />
      <Search
        placeholder={m.modal_ldap_dry_run_search_placeholder()}
        onChange={setSearch}
      />
      <SizedBox height={ThemeSpacing.Xl} />
      {filtered.length === 0 ? (
        <EmptyState
          icon="search"
          title={m.search_empty_common_title()}
          subtitle={m.modal_ldap_dry_run_search_empty_subtitle()}
        />
      ) : (
        <TableBody table={table} maxVisibleRows={10} />
      )}
    </>
  );
};
