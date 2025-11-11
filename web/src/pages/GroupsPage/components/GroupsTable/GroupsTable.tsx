import { useMutation } from '@tanstack/react-query';
import {
  createColumnHelper,
  getCoreRowModel,
  type SortingState,
  useReactTable,
} from '@tanstack/react-table';
import { orderBy } from 'lodash-es';
import { useMemo, useState } from 'react';
import { m } from '../../../../paraglide/messages';
import api from '../../../../shared/api/api';
import type { GroupInfo, User } from '../../../../shared/api/types';
import { Badge } from '../../../../shared/defguard-ui/components/Badge/Badge';
import { Button } from '../../../../shared/defguard-ui/components/Button/Button';
import { IconButtonMenu } from '../../../../shared/defguard-ui/components/IconButtonMenu/IconButtonMenu';
import type { MenuItemProps } from '../../../../shared/defguard-ui/components/Menu/types';
import { tableEditColumnSize } from '../../../../shared/defguard-ui/components/table/consts';
import { TableBody } from '../../../../shared/defguard-ui/components/table/TableBody/TableBody';
import { TableCell } from '../../../../shared/defguard-ui/components/table/TableCell/TableCell';
import { TableTop } from '../../../../shared/defguard-ui/components/table/TableTop/TableTop';
import { isPresent } from '../../../../shared/defguard-ui/utils/isPresent';
import { openModal } from '../../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../../shared/hooks/modalControls/modalTypes';

type Props = {
  groups: GroupInfo[];
  users: User[];
};

type RowData = GroupInfo;

type SortingKeys = 'name';

const columnHelper = createColumnHelper<RowData>();

export const GroupsTable = ({ groups, users }: Props) => {
  const [search, setSearch] = useState('');
  const reservedNames = useMemo(() => groups.map((g) => g.name), [groups]);
  const { mutate: deleteGroup } = useMutation({
    mutationFn: api.group.deleteGroup,
    meta: {
      invalidate: [['group'], ['group-info']],
    },
  });

  const [sortState, setSortState] = useState<SortingState>([
    {
      id: 'name',
      desc: false,
    },
  ]);

  const transformedData = useMemo(() => {
    let data = groups;
    if (search.length) {
      data = data.filter((g) => g.name.toLowerCase().includes(search.toLowerCase()));
    }
    const sorting = sortState[0];
    if (!isPresent(sorting)) return data;
    const sortingId = sorting.id as SortingKeys;
    const sortingDirection = sorting.desc ? 'desc' : 'asc';
    return orderBy(data, (g) => g[sortingId].toLowerCase().replaceAll(' ', ''), [
      sortingDirection,
    ]);
  }, [sortState, groups, search]);

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
        enableSorting: true,
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
      columnHelper.display({
        id: 'edit',
        size: tableEditColumnSize,
        header: '',
        cell: (info) => {
          const rowData = info.row.original;
          const menuItems: MenuItemProps[] = [
            {
              text: m.controls_edit(),
              icon: 'edit',
              onClick: () => {
                openModal(ModalName.CreateEditGroup, {
                  reservedNames,
                  users: users,
                  groupInfo: rowData,
                });
              },
            },
            {
              text: m.controls_delete(),
              icon: 'delete',
              variant: 'danger',
              onClick: () => {
                deleteGroup(rowData.name);
              },
            },
          ];
          return (
            <TableCell>
              <IconButtonMenu
                icon="menu"
                menuItems={[
                  {
                    items: menuItems,
                  },
                ]}
              />
            </TableCell>
          );
        },
      }),
    ],
    [deleteGroup, reservedNames, users],
  );

  const table = useReactTable({
    state: {
      sorting: sortState,
    },
    columns,
    data: transformedData,
    getCoreRowModel: getCoreRowModel(),
    onSortingChange: setSortState,
    manualSorting: true,
  });

  return (
    <>
      <TableTop
        text={m.groups_table_title()}
        onSearch={setSearch}
        initialSearch={search}
        searchPlaceholder={m.groups_search_placeholder()}
      >
        <Button
          iconLeft="add-user"
          text={m.groups_add()}
          onClick={() => {
            openModal(ModalName.CreateEditGroup, {
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
