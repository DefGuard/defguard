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
import type { Device, User } from '../../shared/api/types';
import { useSelectionModal } from '../../shared/components/modals/SelectionModal/useSelectionModal';
import type { SelectionOption } from '../../shared/components/SelectionSection/type';
import { TableValuesListCell } from '../../shared/components/TableValuesListCell/TableValuesListCell';
import { Avatar } from '../../shared/defguard-ui/components/Avatar/Avatar';
import { Badge } from '../../shared/defguard-ui/components/Badge/Badge';
import { Button } from '../../shared/defguard-ui/components/Button/Button';
import type { ButtonProps } from '../../shared/defguard-ui/components/Button/types';
import { EmptyStateFlexible } from '../../shared/defguard-ui/components/EmptyStateFlexible/EmptyStateFlexible';
import { Icon, IconKind } from '../../shared/defguard-ui/components/Icon';
import type { MenuItemsGroup } from '../../shared/defguard-ui/components/Menu/types';
import { Search } from '../../shared/defguard-ui/components/Search/Search';
import { tableEditColumnSize } from '../../shared/defguard-ui/components/table/consts';
import { TableBody } from '../../shared/defguard-ui/components/table/TableBody/TableBody';
import { TableCell } from '../../shared/defguard-ui/components/table/TableCell/TableCell';
import { TableEditCell } from '../../shared/defguard-ui/components/table/TableEditCell/TableEditCell';
import { TableRowContainer } from '../../shared/defguard-ui/components/table/TableRowContainer/TableRowContainer';
import { TableTop } from '../../shared/defguard-ui/components/table/TableTop/TableTop';
import { Snackbar } from '../../shared/defguard-ui/providers/snackbar/snackbar';
import { ThemeVariable } from '../../shared/defguard-ui/types';
import { isPresent } from '../../shared/defguard-ui/utils/isPresent';
import { openModal } from '../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../shared/hooks/modalControls/modalTypes';
import { useApp } from '../../shared/hooks/useApp';
import { useAuth } from '../../shared/hooks/useAuth';
import {
  getGroupsInfoQueryOptions,
  getLicenseInfoQueryOptions,
  getUsersOverviewQueryOptions,
} from '../../shared/query';
import { displayDate } from '../../shared/utils/displayDate';
import { isDeviceOnline, isUserOnline } from '../../shared/utils/userOnlineStatus';
import { useAddUserModal } from './modals/AddUserModal/useAddUserModal';

type RowData = User;

const columnHelper = createColumnHelper<RowData>();

export const UsersTable = () => {
  const { data: users } = useSuspenseQuery(getUsersOverviewQueryOptions);
  const { data: license } = useSuspenseQuery(getLicenseInfoQueryOptions);
  const appInfo = useApp((s) => s.appInfo);
  const authUsername = useAuth((s) => s.user?.username);
  const reservedEmails = useMemo(() => users.map((u) => u.email.toLowerCase()), [users]);
  const reservedUsernames = useMemo(() => users.map((u) => u.username), [users]);

  const addButtonProps = useMemo(
    (): ButtonProps => ({
      variant: 'primary',
      text: m.users_add(),
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

  const tableFilterMessages = useMemo(
    () => ({
      searchPlaceholder: m.controls_search(),
      clearButton: m.controls_reset(),
      applyButton: m.controls_submit(),
      emptyState: m.search_empty_common_title(),
    }),
    [],
  );

  const { data: groups } = useQuery(getGroupsInfoQueryOptions);

  const groupsOptions = useMemo(
    (): SelectionOption<string>[] =>
      groups?.map((g) => ({
        id: g.name,
        label: g.name,
      })) ?? [],
    [groups?.map, groups],
  );

  const { mutate: editUser } = useMutation({
    mutationFn: api.user.editUser,
    meta: {
      invalidate: [['user-overview'], ['user'], ['activity-log']],
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
        meta: {
          flex: true,
        },
        cell: (info) => {
          const rowData = info.row.original;
          const online = isUserOnline(rowData);

          return (
            <TableCell>
              <Avatar
                size="default"
                variant="initials"
                firstName={rowData.first_name}
                lastName={rowData.last_name}
                online={online}
              />
              <span>{info.getValue()}</span>
            </TableCell>
          );
        },
      }),
      columnHelper.accessor('is_active', {
        header: m.col_status(),
        size: 100,
        minSize: 100,
        cell: (info) => (
          <TableCell>
            {info.getValue() ? (
              <Badge variant="success" text={m.state_active()} />
            ) : (
              <Badge variant="critical" text={m.state_disabled()} />
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
      columnHelper.accessor('email', {
        header: m.form_label_email(),
        size: 200,
        minSize: 150,
        enableSorting: true,
        sortingFn: 'text',
        cell: (info) => (
          <TableCell>
            <span>{info.getValue()}</span>
          </TableCell>
        ),
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
      columnHelper.accessor('mfa_enabled', {
        header: m.users_col_mfa(),
        size: 56,
        minSize: 56,
        cell: (info) => (
          <TableCell className="cell-with-check-icons">
            {info.getValue() ? (
              <Icon icon="check-filled" staticColor={ThemeVariable.FgSuccess} />
            ) : null}
          </TableCell>
        ),
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
          const accountStatusMenuGroup: MenuItemsGroup = {
            items: [
              {
                text: rowData.is_active
                  ? m.users_row_menu_disable()
                  : m.users_row_menu_enable(),
                icon: rowData.is_active ? 'disabled' : 'check-circle',
                testId: 'change-account-status',
                onClick: () => {
                  if (rowData.is_active) {
                    openModal(ModalName.ConfirmAction, {
                      title: m.users_modal_disable_title(),
                      contentMd: m.users_modal_disable_content({ name: rowData.name }),
                      actionPromise: () =>
                        api.user.activeStateChange({
                          active: false,
                          username: rowData.username,
                        }),
                      invalidateKeys: [['user-overview'], ['user']],
                      submitProps: {
                        text: m.users_row_menu_disable(),
                        variant: 'critical',
                      },
                      onSuccess: () => Snackbar.default(m.users_disable_success()),
                      onError: () => Snackbar.error(m.users_disable_error()),
                    });
                  } else {
                    openModal(ModalName.ConfirmAction, {
                      title: m.users_modal_enable_title(),
                      contentMd: m.users_modal_enable_content({ name: rowData.name }),
                      actionPromise: () =>
                        api.user.activeStateChange({
                          active: true,
                          username: rowData.username,
                        }),
                      invalidateKeys: [['user-overview'], ['user']],
                      submitProps: {
                        text: m.users_row_menu_enable(),
                      },
                      onSuccess: () => Snackbar.default(m.users_enable_success()),
                      onError: () => Snackbar.error(m.users_enable_error()),
                    });
                  }
                },
              },
            ],
          };

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
                      adminForm: rowData.username !== authUsername,
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
                {
                  text: m.users_row_menu_ip_settings(),
                  icon: IconKind.Gateway,
                  testId: 'assign-ip',
                  onClick: async () => {
                    const response = await api.device.getUserDeviceIps(rowData.username);
                    openModal(ModalName.AssignUserIP, {
                      user: rowData,
                      locationData: response.data,
                      hasDevices: rowData.devices.length > 0,
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
            accountStatusMenuGroup,
            {
              items: [
                {
                  text: m.users_row_menu_delete(),
                  icon: 'delete',
                  variant: 'danger',
                  onClick: () => {
                    openModal(ModalName.ConfirmAction, {
                      title: m.modal_delete_user_title(),
                      contentMd: m.modal_delete_user_body({ name: rowData.name }),
                      actionPromise: () => api.user.deleteUser(rowData.username),
                      invalidateKeys: [['user-overview'], ['user'], ['enterprise_info']],
                      submitProps: {
                        text: m.users_row_menu_delete(),
                        variant: 'critical',
                      },
                      onSuccess: () => Snackbar.default(m.modal_delete_user_success()),
                      onError: () => Snackbar.error(m.modal_delete_user_error()),
                    });
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
                        Snackbar.error(m.failed_to_start_enrollment());
                        console.error(error);
                      });
                  },
                },
              ],
            });
          }
          if (rowData.enrolled) {
            menuItems.splice(1, 0, {
              items: [
                {
                  text: m.user_row_menu_add_new_device(),
                  icon: IconKind.AddDevice,
                  onClick: () => {
                    openModal(ModalName.AddNewDevice, rowData);
                  },
                },
              ],
            });
          }
          if (rowData.mfa_enabled) {
            accountStatusMenuGroup.items.splice(1, 0, {
              text: m.users_row_menu_disable_mfa(),
              icon: 'disable-mfa',
              onClick: () => {
                openModal(ModalName.ConfirmAction, {
                  title: m.users_modal_disable_mfa_title(),
                  contentMd: m.users_modal_disable_mfa_content({
                    name: rowData.name,
                  }),
                  actionPromise: () => api.user.disableMfa(rowData.username),
                  invalidateKeys: [
                    ['user-overview'],
                    ['user'],
                    ['session-info'],
                    ['me'],
                    ['activity-log'],
                  ],
                  submitProps: {
                    text: m.users_row_menu_disable_mfa(),
                    variant: 'critical',
                  },
                  onSuccess: () => Snackbar.default(m.users_disable_mfa_success()),
                  onError: () => Snackbar.error(m.users_disable_mfa_error()),
                });
              },
            });
          }

          return <TableEditCell menuItems={menuItems} />;
        },
      }),
    ],
    [
      navigate,
      reservedEmails,
      reservedUsernames,
      groupsOptions,
      handleEditGroups,
      groups,
      appInfo,
      authUsername,
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
    ],
    [],
  );

  const makeDeviceRowMenu = useCallback(
    (
      device: Device,
      username: string,
      reservedDeviceNames: string[],
      reservedPubkeys: string[],
    ): MenuItemsGroup[] => [
      {
        items: [
          {
            text: m.controls_edit(),
            icon: 'edit',
            onClick: () => {
              openModal(ModalName.EditUserDevice, {
                device,
                reservedNames: reservedDeviceNames,
                reservedPubkeys,
                username,
              });
            },
          },
          {
            text: m.profile_devices_menu_ip_settings(),
            icon: 'gateway',
            testId: 'assign-device-ip',
            onClick: () => {
              api.device
                .getDeviceIps(username, device.id)
                .then(({ data: locationData }) => {
                  openModal(ModalName.AssignUserDeviceIP, {
                    device,
                    username,
                    locationData,
                  });
                })
                .catch((error) => {
                  Snackbar.error(m.profile_devices_ip_settings_load_failed());
                  console.error(error);
                });
            },
          },
          {
            text: m.profile_devices_menu_show_config(),
            onClick: () => {
              api.device.getDeviceConfigs(device).then((modalData) => {
                openModal(ModalName.UserDeviceConfig, modalData);
              });
            },
            icon: 'config',
          },
          {
            text: m.controls_delete(),
            onClick: () => {
              openModal(ModalName.ConfirmAction, {
                title: m.modal_delete_user_device_title(),
                contentMd: m.modal_delete_user_device_body({ name: device.name }),
                actionPromise: () => api.device.deleteDevice(device.id),
                invalidateKeys: [['user-overview'], ['user'], ['network']],
                submitProps: { text: m.controls_delete(), variant: 'critical' },
                onSuccess: () => Snackbar.default(m.user_device_delete_success()),
                onError: () => Snackbar.error(m.user_device_delete_failed()),
              });
            },
            variant: 'danger',
            icon: 'delete',
          },
        ],
      },
    ],
    [],
  );

  const renderExpanded = useCallback(
    (row: Row<RowData>, isLast = false) => {
      const username = row.original.username;
      const reservedDeviceNames = row.original.devices.map((d) => d.name);
      const reservedPubkeys = row.original.devices.map((d) => d.wireguard_pubkey);
      return row.original.devices.map((device, deviceIndex) => {
        const lastRow = isLast && deviceIndex === row.original.devices.length - 1;
        const deviceOnline = isDeviceOnline(device);
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
        const menuItems = makeDeviceRowMenu(
          device,
          username,
          reservedDeviceNames,
          reservedPubkeys,
        );
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
            <TableCell>
              <div className="expanded-device-icon-wrapper">
                <Icon icon="devices" staticColor={ThemeVariable.FgNeutral} />
                {deviceOnline && (
                  <span className="expanded-device-online-indicator" aria-hidden="true" />
                )}
              </div>
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
            <TableEditCell menuItems={menuItems} />
          </TableRowContainer>
        );
      });
    },
    [makeDeviceRowMenu],
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
    meta: {
      filterMessages: tableFilterMessages,
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
        title={m.users_empty_title()}
        subtitle={m.users_empty_subtitle()}
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
              text={m.users_bulk_assign_to_groups()}
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
        <EmptyStateFlexible
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
