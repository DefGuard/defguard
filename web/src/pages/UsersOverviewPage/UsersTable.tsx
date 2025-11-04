import {
  createColumnHelper,
  getCoreRowModel,
  type SortingState,
  useReactTable,
} from '@tanstack/react-table';
import { useMemo, useState } from 'react';
import { m } from '../../paraglide/messages';
import type { User } from '../../shared/api/types';
import { Avatar } from '../../shared/defguard-ui/components/Avatar/Avatar';
import { IconButtonMenu } from '../../shared/defguard-ui/components/IconButtonMenu/IconButtonMenu';
import { tableEditColumnSize } from '../../shared/defguard-ui/components/table/consts';
import { TableBody } from '../../shared/defguard-ui/components/table/TableBody/TableBody';
import { TableCell } from '../../shared/defguard-ui/components/table/TableCell/TableCell';
import { TableTop } from '../../shared/defguard-ui/components/table/TableTop/TableTop';

type Props = {
  users: User[];
};

type RowData = User;

const columnHelper = createColumnHelper<RowData>();

export const UsersTable = ({ users }: Props) => {
  const transformedData = useMemo(() => users, [users]);
  const [sortingState, setSortingState] = useState<SortingState>([
    {
      id: 'name',
      desc: false,
    },
  ]);

  const columns = useMemo(
    () => [
      columnHelper.display({
        id: 'name',
        header: m.users_col_name(),
        enableSorting: true,
        meta: {
          flex: true,
        },
        cell: (info) => {
          const rowData = info.row.original;
          const name = `${rowData.first_name} ${rowData.last_name}`;
          return (
            <TableCell>
              <Avatar
                size="default"
                variant="initials"
                firstName={rowData.first_name}
                lastName={rowData.last_name}
              />
              <span>{name}</span>
            </TableCell>
          );
        },
      }),
      columnHelper.accessor('username', {
        header: m.users_col_login(),
        size: 170,
        enableSorting: false,
        cell: (info) => (
          <TableCell>
            <span>{info.getValue()}</span>
          </TableCell>
        ),
      }),
      columnHelper.accessor('phone', {
        size: 175,
        header: m.users_col_phone(),
        enableSorting: false,
        cell: (info) => (
          <TableCell>
            <span>{info.getValue()}</span>
          </TableCell>
        ),
      }),
      columnHelper.accessor('groups', {
        header: m.users_col_groups(),
        size: 370,
        enableSorting: false,
        cell: (info) => (
          <TableCell>
            <span>{info.getValue().join(', ')}</span>
          </TableCell>
        ),
      }),
      columnHelper.display({
        id: 'edit',
        size: tableEditColumnSize,
        header: '',
        enableSorting: false,
        cell: (_info) => {
          // const _rowData = info.row.original;
          return (
            <TableCell>
              <IconButtonMenu icon="menu" menuItems={[]} />
            </TableCell>
          );
        },
      }),
    ],
    [],
  );

  const table = useReactTable({
    state: {
      sorting: sortingState,
    },
    columns,
    data: transformedData,
    getCoreRowModel: getCoreRowModel(),
    manualSorting: true,
    onSortingChange: setSortingState,
  });

  return (
    <>
      <TableTop text={m.users_header_title()}></TableTop>
      <TableBody table={table} />
    </>
  );
};
