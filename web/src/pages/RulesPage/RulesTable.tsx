import {
  createColumnHelper,
  getCoreRowModel,
  useReactTable,
} from '@tanstack/react-table';
import { useMemo, useState } from 'react';
import { m } from '../../paraglide/messages';
import type { AclRule } from '../../shared/api/types';
import { Button } from '../../shared/defguard-ui/components/Button/Button';
import type { ButtonProps } from '../../shared/defguard-ui/components/Button/types';
import { EmptyStateFlexible } from '../../shared/defguard-ui/components/EmptyStateFlexible/EmptyStateFlexible';
import { Search } from '../../shared/defguard-ui/components/Search/Search';
import { TableBody } from '../../shared/defguard-ui/components/table/TableBody/TableBody';
import { TableCell } from '../../shared/defguard-ui/components/table/TableCell/TableCell';
import { TableTop } from '../../shared/defguard-ui/components/table/TableTop/TableTop';

type RowData = AclRule;

const columnHelper = createColumnHelper<RowData>();

type Props = {
  data: AclRule[];
  title: string;
  buttonProps: ButtonProps;
  enableSearch?: boolean;
};

export const RulesTable = ({ title, buttonProps, enableSearch, data }: Props) => {
  const [search, setSearch] = useState('');

  const columns = useMemo(
    () => [
      columnHelper.accessor('name', {
        header: 'Rule name',
        cell: (info) => (
          <TableCell>
            <span>{info.getValue()}</span>
          </TableCell>
        ),
      }),
    ],
    [],
  );

  const visibleRules = useMemo(() => {
    let res = data;
    if (search.length) {
      res = res.filter((rule) => rule.name.toLowerCase().includes(search.toLowerCase()));
    }
    return res;
  }, [search, data]);

  const table = useReactTable({
    columns,
    data: visibleRules,
    enableRowSelection: false,
    columnResizeMode: 'onChange',
    getCoreRowModel: getCoreRowModel(),
  });

  if (data.length === 0) return null;

  return (
    <>
      <TableTop text={title}>
        {enableSearch && (
          <Search placeholder={m.controls_search()} value={search} onChange={setSearch} />
        )}
        <Button {...buttonProps} />
      </TableTop>
      {visibleRules.length > 0 && <TableBody table={table} />}
      {visibleRules.length === 0 && (
        <EmptyStateFlexible
          icon="search"
          title={m.search_empty_common_title()}
          subtitle={m.search_empty_common_subtitle()}
        />
      )}
    </>
  );
};
