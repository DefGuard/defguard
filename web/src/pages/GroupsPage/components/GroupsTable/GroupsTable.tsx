import {
  createColumnHelper,
  getCoreRowModel,
  useReactTable,
} from '@tanstack/react-table';
import { useMemo } from 'react';
import { m } from '../../../../paraglide/messages';
import type { GroupInfo, User } from '../../../../shared/api/types';
import { Badge } from '../../../../shared/defguard-ui/components/Badge/Badge';
import { Button } from '../../../../shared/defguard-ui/components/Button/Button';
import { TableBody } from '../../../../shared/defguard-ui/components/table/TableBody/TableBody';
import { TableCell } from '../../../../shared/defguard-ui/components/table/TableCell/TableCell';
import { TableTop } from '../../../../shared/defguard-ui/components/table/TableTop/TableTop';
import { openModal } from '../../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../../shared/hooks/modalControls/modalTypes';

type Props = {
  groups: GroupInfo[];
  users: User[];
};

type RowData = GroupInfo;

const columnHelper = createColumnHelper<RowData>();

export const GroupsTable = ({ groups, users }: Props) => {
  const columns = useMemo(
    () => [
      columnHelper.accessor('name', {
        header: m.groups_col_name(),
        cell: (info) => (
          <TableCell>
            <span>{info.getValue()}</span>
          </TableCell>
        ),
        meta: {
          flex: true,
        },
      }),
      columnHelper.display({
        size: 108,
        id: 'users_count',
        header: m.groups_col_users_count(),
        cell: (info) => {
          const row = info.row.original;
          return (
            <TableCell>
              <span>{row.members.length}</span>
            </TableCell>
          );
        },
      }),
      columnHelper.accessor('is_admin', {
        size: 108,
        header: m.groups_col_type(),
        cell: (info) => (
          <TableCell>
            {info.getValue() ? (
              <Badge variant="success" text={m.groups_type_admin()} />
            ) : (
              <Badge text={m.groups_type_user()} />
            )}
          </TableCell>
        ),
      }),
      columnHelper.accessor('vpn_locations', {
        size: 550,
        header: m.groups_col_locations(),
        cell: (info) => (
          <TableCell>
            <span>{info.getValue().join(', ')}</span>
          </TableCell>
        ),
      }),
    ],
    [],
  );

  const table = useReactTable({
    columns,
    data: groups,
    getCoreRowModel: getCoreRowModel(),
  });

  return (
    <>
      <TableTop text={m.groups_table_title()}>
        <Button
          iconLeft="add-user"
          text={m.groups_add()}
          onClick={() => {
            const reservedNames = groups.map((g) => g.name);
            openModal(ModalName.AddGroup, {
              reservedNames,
              users,
            });
          }}
        />
      </TableTop>
      <TableBody table={table} />
    </>
  );
};
