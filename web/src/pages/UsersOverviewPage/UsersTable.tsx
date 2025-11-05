import {
  createColumnHelper,
  getCoreRowModel,
  getExpandedRowModel,
  type Row,
  type SortingState,
  useReactTable,
} from '@tanstack/react-table';
import clsx from 'clsx';
import { orderBy } from 'lodash-es';
import { type CSSProperties, useCallback, useMemo, useState } from 'react';
import { m } from '../../paraglide/messages';
import type { UsersListItem } from '../../shared/api/types';
import { Avatar } from '../../shared/defguard-ui/components/Avatar/Avatar';
import { Badge } from '../../shared/defguard-ui/components/Badge/Badge';
import { Button } from '../../shared/defguard-ui/components/Button/Button';
import { Icon } from '../../shared/defguard-ui/components/Icon';
import { IconButtonMenu } from '../../shared/defguard-ui/components/IconButtonMenu/IconButtonMenu';
import {
  tableActionColumnSize,
  tableEditColumnSize,
} from '../../shared/defguard-ui/components/table/consts';
import { TableBody } from '../../shared/defguard-ui/components/table/TableBody/TableBody';
import { TableCell } from '../../shared/defguard-ui/components/table/TableCell/TableCell';
import { TableExpandCell } from '../../shared/defguard-ui/components/table/TableExpandCell/TableExpandCell';
import { TableRowContainer } from '../../shared/defguard-ui/components/table/TableRowContainer/TableRowContainer';
import { TableTop } from '../../shared/defguard-ui/components/table/TableTop/TableTop';
import { isPresent } from '../../shared/defguard-ui/utils/isPresent';
import { displayDate } from '../../shared/utils/displayDate';
import { useAddUserModal } from './modals/AddUserModal/useAddUserModal';

type Props = {
  users: UsersListItem[];
};

type RowData = UsersListItem;

const columnHelper = createColumnHelper<RowData>();

export const UsersTable = ({ users }: Props) => {
  const [sortingState, setSortingState] = useState<SortingState>([
    {
      id: 'name',
      desc: false,
    },
  ]);

  const transformedData = useMemo(() => {
    const sorting = sortingState[0];
    if (!sorting) return users;
    const { id, desc } = sorting;
    const direction = desc ? 'desc' : 'asc';
    const orderedDevices = users.map((user) => ({
      ...user,
      devices: orderBy(user.devices, [id], [direction]),
    }));
    return orderBy(
      orderedDevices,
      (user) => `${user.first_name}${user.last_name}`.toLowerCase(),
      [direction],
    );
  }, [users, sortingState]);

  const columns = useMemo(
    () => [
      columnHelper.display({
        id: 'expand',
        header: '',
        size: tableActionColumnSize,
        cell: (info) => <TableExpandCell row={info.row} />,
      }),
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
      columnHelper.accessor('is_active', {
        header: m.users_col_status(),
        size: 100,
        cell: (info) => (
          <TableCell>
            {info.getValue() ? (
              <Badge variant="success" text={m.misc_active()} />
            ) : (
              <Badge variant="critical" text={m.misc_disabled()} />
            )}
          </TableCell>
        ),
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
        cell: (info) => {
          const phone = info.getValue();
          const display = isPresent(phone) && phone.length ? phone : '~';
          return (
            <TableCell>
              <span>{display}</span>
            </TableCell>
          );
        },
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
              <IconButtonMenu
                icon="menu"
                menuItems={[
                  {
                    items: [],
                  },
                ]}
              />
            </TableCell>
          );
        },
      }),
    ],
    [],
  );

  const expandedHeader = useMemo(
    () => [
      '',
      m.users_col_assigned(),
      '',
      m.users_col_ip(),
      m.users_col_connected_through(),
      m.users_col_connected_date(),
      '',
    ],
    [],
  );

  const renderExpanded = useCallback(
    (row: Row<RowData>, rowStyles: CSSProperties, isLast = false) =>
      row.original.devices.map((device) => {
        const latestNetwork = orderBy(
          device.networks.filter((n) => isPresent(n.last_connected_at)),
          (d) => d.last_connected_at,
          ['desc'],
        )[0];
        const neverConnected = m.profile_devices_col_never_connected();
        const ip = latestNetwork?.last_connected_ip ?? neverConnected;
        const locationName = latestNetwork?.last_connected_at
          ? latestNetwork.network_name
          : neverConnected;
        const connectionDate = latestNetwork?.last_connected_at
          ? displayDate(latestNetwork.last_connected_at)
          : neverConnected;
        return (
          <TableRowContainer
            className={clsx({ last: isLast })}
            key={device.id}
            style={rowStyles}
          >
            <TableCell alignContent="center" noPadding>
              <Icon icon="enter" />
            </TableCell>
            <TableCell className="device-name-cell">
              <Icon icon="devices" />
              <span>{device.name}</span>
            </TableCell>
            <TableCell empty />
            <TableCell>
              <span>{ip}</span>
            </TableCell>
            <TableCell>
              <span>{locationName}</span>
            </TableCell>
            <TableCell>
              <span>{connectionDate}</span>
            </TableCell>
            <TableCell empty />
          </TableRowContainer>
        );
      }),
    [],
  );

  const table = useReactTable({
    state: {
      sorting: sortingState,
    },
    columns,
    data: transformedData,
    manualSorting: true,
    getCoreRowModel: getCoreRowModel(),
    getExpandedRowModel: getExpandedRowModel(),
    getRowCanExpand: (row) => row.original.devices.length > 0,
    onSortingChange: setSortingState,
  });

  return (
    <>
      <TableTop text={m.users_header_title()}>
        <Button
          iconLeft="add-user"
          text={m.users_add()}
          onClick={() => {
            const reservedEmails = users.map((u) => u.email.toLowerCase());
            const reservedUsernames = users.map((u) => u.username);
            useAddUserModal.getState().open({
              reservedEmails,
              reservedUsernames,
            });
          }}
        />
      </TableTop>
      <TableBody
        table={table}
        renderExpandedRow={renderExpanded}
        expandedHeaders={expandedHeader}
      />
    </>
  );
};
