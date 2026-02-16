import { useMutation, useQuery, useSuspenseQuery } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import {
  type ColumnFiltersState,
  createColumnHelper,
  getCoreRowModel,
  getExpandedRowModel,
  getFilteredRowModel,
  getSortedRowModel,
  type Row,
  type RowSelectionState,
  useReactTable,
} from '@tanstack/react-table';
import clsx from 'clsx';
import { orderBy } from 'lodash-es';
import { useCallback, useMemo, useState } from 'react';
import { m } from '../../paraglide/messages';
import api from '../../shared/api/api';
import type { UsersListItem } from '../../shared/api/types';
import { useSelectionModal } from '../../shared/components/modals/SelectionModal/useSelectionModal';
import type { SelectionOption } from '../../shared/components/SelectionSection/type';
import { TableValuesListCell } from '../../shared/components/TableValuesListCell/TableValuesListCell';
import { Avatar } from '../../shared/defguard-ui/components/Avatar/Avatar';
import { Badge } from '../../shared/defguard-ui/components/Badge/Badge';
import { Button } from '../../shared/defguard-ui/components/Button/Button';
import type { ButtonProps } from '../../shared/defguard-ui/components/Button/types';
import { EmptyState } from '../../shared/defguard-ui/components/EmptyState/EmptyState';
import { EmptyStateFlexible } from '../../shared/defguard-ui/components/EmptyStateFlexible/EmptyStateFlexible';
import { Icon, IconKind } from '../../shared/defguard-ui/components/Icon';
import { IconButtonMenu } from '../../shared/defguard-ui/components/IconButtonMenu/IconButtonMenu';
import type { MenuItemsGroup } from '../../shared/defguard-ui/components/Menu/types';
import { Search } from '../../shared/defguard-ui/components/Search/Search';
import { tableEditColumnSize } from '../../shared/defguard-ui/components/table/consts';
import { TableBody } from '../../shared/defguard-ui/components/table/TableBody/TableBody';
import { TableCell } from '../../shared/defguard-ui/components/table/TableCell/TableCell';
import { TableFlexCell } from '../../shared/defguard-ui/components/table/TableFlexCell/TableFlexCell';
import { TableRowContainer } from '../../shared/defguard-ui/components/table/TableRowContainer/TableRowContainer';
import { TableTop } from '../../shared/defguard-ui/components/table/TableTop/TableTop';
import { Snackbar } from '../../shared/defguard-ui/providers/snackbar/snackbar';
import { isPresent } from '../../shared/defguard-ui/utils/isPresent';
import { openModal } from '../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../shared/hooks/modalControls/modalTypes';
import { useApp } from '../../shared/hooks/useApp';
import {
  getGroupsInfoQueryOptions,
  getLicenseInfoQueryOptions,
  getUsersOverviewQueryOptions,
} from '../../shared/query';
import { displayDate } from '../../shared/utils/displayDate';
import { useAddUserModal } from './modals/AddUserModal/useAddUserModal';

type RowData = UsersListItem;

const columnHelper = createColumnHelper<RowData>();

export const UsersTable = () => {
  const { data: users } = useSuspenseQuery(getUsersOverviewQueryOptions);
  const { data: license } = useSuspenseQuery(getLicenseInfoQueryOptions);
  const appInfo = useApp((s) => s.appInfo);
  const reservedEmails = useMemo(() => users.map((u) => u.email.toLowerCase()), [users]);
  const reservedUsernames = useMemo(() => users.map((u) => u.username), [users]);

  const addButtonProps = useMemo(
    (): ButtonProps => ({
      variant: 'primary',
      text: 'Add new user',
      iconLeft: 'add-user',
      testId: 'add-user',
      onClick: () => {
        if (
          license?.limits &&
          license.limits.users.current === license.limits.users.limit
        ) {
          openModal(ModalName.LimitReached);
        } else {
          useAddUserModal.getState().open({
            reservedEmails,
            reservedUsernames,
          });
        }
      },
    }),
    [reservedEmails, reservedUsernames, license],
  );

  const [search, setSearch] = useState('');
  const [columnFilters, setColumnFilters] = useState<ColumnFiltersState>([]);

  const { data: groups } = useQuery(getGroupsInfoQueryOptions);

  const groupsOptions = useMemo(
    (): SelectionOption<string>[] =>
      groups?.map((g) => ({
        id: g.name,
        label: g.name,
      })) ?? [],
    [groups?.map, groups],
  );

  const { mutate: deleteUser } = useMutation({
    mutationFn: api.user.deleteUser,
    meta: {
      invalidate: [['user-overview'], ['user'], ['enterprise_info']],
    },
  });

  const { mutate: changeAccountActiveState } = useMutation({
    mutationFn: api.user.activeStateChange,
    meta: {
      invalidate: [['user-overview'], ['user']],
    },
  });

  const { mutate: editUser } = useMutation({
    mutationFn: api.user.editUser,
    meta: {
      invalidate: [['user-overview'], ['user']],
    },
  });

  const handleEditGroups = useCallback(
    async (user: RowData, groups: string[]) => {
      const freshUser = (await api.user.getUser(user.username)).data.user;
      freshUser.groups = groups;
      editUser({
        username: freshUser.username,
        body: freshUser,
      });
    },
    [editUser],
  );

  const navigate = useNavigate({ from: '/users' });
  const [selected, setSelected] = useState<RowSelectionState>({});

  const transformedData = useMemo(() => {
    let data = users;
    if (search.length) {
      data = data.filter(
        (u) =>
          u.first_name.toLowerCase().includes(search.toLowerCase()) ||
          u.last_name.toLowerCase().includes(search.toLowerCase()),
      );
    }
    return data;
  }, [users, search.length, search.toLowerCase, search]);

  const columns = useMemo(
    () => [
      columnHelper.accessor('name', {
        header: m.users_col_name(),
        enableSorting: true,
        sortingFn: 'text',
        minSize: 250,
        cell: (info) => {
          const rowData = info.row.original;
          return (
            <TableCell>
              <Avatar
                size="default"
                variant="initials"
                firstName={rowData.first_name}
                lastName={rowData.last_name}
              />
              <span>{info.getValue()}</span>
            </TableCell>
          );
        },
      }),
      columnHelper.accessor('is_active', {
        header: m.users_col_status(),
        size: 100,
        minSize: 100,
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
        minSize: 100,
        enableSorting: true,
        sortingFn: 'text',
        cell: (info) => (
          <TableCell>
            <span>{info.getValue()}</span>
          </TableCell>
        ),
      }),
      columnHelper.accessor('phone', {
        size: 175,
        minSize: 175,
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
        minSize: 200,
        enableSorting: false,
        enableColumnFilter: isPresent(groups),
        filterFn: 'arrIncludesSome',
        meta: {
          filterOptions:
            groups?.map(
              (group): SelectionOption<string> => ({
                id: group.name,
                label: group.name,
              }),
            ) ?? [],
        },
        cell: (info) => <TableValuesListCell values={info.getValue()} />,
      }),
      columnHelper.accessor('enrolled', {
        header: m.users_col_enrolled(),
        size: 150,
        minSize: 125,
        cell: (info) => (
          <TableCell>
            {info.getValue() ? (
              <Badge text={m.state_enrolled()} />
            ) : (
              <Badge text={m.state_pending()} icon="pending" variant="warning" showIcon />
            )}
          </TableCell>
        ),
      }),
      columnHelper.display({
        id: 'edit',
        size: tableEditColumnSize,
        header: '',
        enableSorting: false,
        enableResizing: false,
        cell: (info) => {
          const rowData = info.row.original;

          const menuItems: MenuItemsGroup[] = [
            {
              items: [
                {
                  text: m.users_row_menu_edit(),
                  icon: 'edit',
                  onClick: () => {
                    openModal(ModalName.EditUserModal, {
                      user: rowData,
                      reservedEmails,
                      reservedUsernames,
                    });
                  },
                },
                {
                  text: m.users_row_menu_change_password(),
                  icon: 'lock-open',
                  testId: 'change-password',
                  onClick: () => {
                    openModal(ModalName.ChangePassword, {
                      adminForm: true,
                      user: rowData,
                    });
                  },
                },
                {
                  text: m.users_row_menu_go_profile(),
                  icon: 'profile',
                  onClick: () => {
                    navigate({
                      to: '/user/$username',
                      params: {
                        username: rowData.username,
                      },
                    });
                  },
                },
                {
                  text: m.users_row_menu_edit_groups(),
                  icon: 'add-group',
                  testId: 'edit-groups',
                  onClick: () => {
                    useSelectionModal.setState({
                      isOpen: true,
                      options: groupsOptions,
                      title: m.modal_edit_user_groups_title(),
                      selected: new Set(rowData.groups),
                      onSubmit: (selected) => {
                        handleEditGroups(rowData, selected as string[]);
                      },
                    });
                  },
                },
              ],
            },
            {
              items: [
                {
                  text: m.users_row_menu_add_auth(),
                  icon: 'key',
                  onClick: () => {
                    openModal(ModalName.AddAuthKey, {
                      username: rowData.username,
                    });
                  },
                },
              ],
            },
            {
              items: [
                {
                  text: rowData.is_active
                    ? m.users_row_menu_disable()
                    : m.users_row_menu_enable(),
                  icon: rowData.is_active ? 'disabled' : 'check-circle',
                  testId: 'change-account-status',
                  onClick: () => {
                    changeAccountActiveState({
                      active: !rowData.is_active,
                      username: rowData.username,
                    });
                  },
                },
              ],
            },
            {
              items: [
                {
                  text: m.users_row_menu_delete(),
                  icon: 'delete',
                  variant: 'danger',
                  onClick: () => {
                    deleteUser(rowData.username);
                  },
                },
              ],
            },
          ];

          if (!rowData.enrolled) {
            menuItems.splice(1, 0, {
              items: [
                {
                  text: m.users_row_menu_initiate_self_enrollment(),
                  icon: IconKind.AddUser,
                  onClick: () => {
                    api.user
                      .startEnrollment({
                        send_enrollment_notification: false,
                        username: rowData.username,
                      })
                      .then((response) => {
                        openModal(ModalName.SelfEnrollmentToken, {
                          user: rowData,
                          appInfo,
                          enrollmentResponse: response.data,
                        });
                      })
                      .catch((error) => {
                        Snackbar.error('Failed to initiate enrollment');
                        console.error(error);
                      });
                  },
                },
              ],
            });
          }

          return (
            <TableCell>
              <IconButtonMenu icon="menu" menuItems={menuItems} />
            </TableCell>
          );
        },
      }),
    ],
    [
      navigate,
      reservedEmails,
      reservedUsernames,
      changeAccountActiveState,
      deleteUser,
      groupsOptions,
      handleEditGroups,
      groups,
      appInfo,
    ],
  );

  const expandedHeader = useMemo(
    () => [
      m.users_col_assigned(),
      '',
      m.users_col_ip(),
      m.users_col_connected_through(),
      m.users_col_connected_date(),
      '',
      '',
    ],
    [],
  );

  const renderExpanded = useCallback(
    (row: Row<RowData>, isLast = false) =>
      row.original.devices.map((device, deviceIndex) => {
        const lastRow = isLast && deviceIndex === row.original.devices.length - 1;
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
            className={clsx({ last: lastRow })}
            key={device.id}
            assignColumnSizing
          >
            <TableCell empty />
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
            <TableCell empty />
            <TableFlexCell />
          </TableRowContainer>
        );
      }),
    [],
  );

  const table = useReactTable({
    initialState: {
      sorting: [
        {
          id: 'name',
          desc: false,
        },
      ],
    },
    state: {
      rowSelection: selected,
      columnFilters: columnFilters,
    },
    columns,
    data: transformedData,
    enableRowSelection: true,
    enableExpanding: true,
    columnResizeMode: 'onChange',
    onColumnFiltersChange: setColumnFilters,
    getFilteredRowModel: getFilteredRowModel(),
    onRowSelectionChange: setSelected,
    getSortedRowModel: getSortedRowModel(),
    getCoreRowModel: getCoreRowModel(),
    getExpandedRowModel: getExpandedRowModel(),
    getRowCanExpand: (row) => row.original.devices.length > 0,
  });

  if (users.length === 0)
    return (
      <EmptyStateFlexible
        title={`No users here yet.`}
        subtitle={`Add users by clicking the button below.`}
        primaryAction={addButtonProps}
      />
    );

  return (
    <>
      <TableTop text={m.users_header_title()}>
        {(table.getIsSomeRowsSelected() || table.getIsAllRowsSelected()) &&
          isPresent(groups) && (
            <Button
              variant="outlined"
              text="Assign to a group"
              iconLeft="add-group"
              testId="bulk-assign"
              onClick={() => {
                const selected = table
                  .getSelectedRowModel()
                  .rows.map((row) => row.original.id);
                openModal(ModalName.AssignGroupsToUsers, {
                  groups,
                  users: selected,
                });
                table.resetRowSelection();
              }}
            />
          )}
        <Search
          placeholder={m.users_search_placeholder()}
          initialValue={search}
          onChange={setSearch}
        />
        <Button {...addButtonProps} />
      </TableTop>
      {transformedData.length === 0 && search.length > 0 && (
        <EmptyState
          icon="search"
          title={m.search_empty_common_title()}
          subtitle={m.search_empty_common_subtitle()}
        />
      )}
      {transformedData.length > 0 && (
        <TableBody
          table={table}
          renderExpandedRow={renderExpanded}
          expandedHeaders={expandedHeader}
        />
      )}
    </>
  );
};
