import './style.scss';
import {
  createColumnHelper,
  getCoreRowModel,
  getExpandedRowModel,
  getSortedRowModel,
  type Row,
  useReactTable,
} from '@tanstack/react-table';
import clsx from 'clsx';
import orderBy from 'lodash-es/orderBy';
import { useCallback, useMemo } from 'react';
import { m } from '../../../../../../../paraglide/messages';
import api from '../../../../../../../shared/api/api';
import type { UserDevice } from '../../../../../../../shared/api/types';
import { useAddUserDeviceModal } from '../../../../../../../shared/components/modals/AddUserDeviceModal/store/useAddUserDeviceModal';
import { Badge } from '../../../../../../../shared/defguard-ui/components/Badge/Badge';
import { Button } from '../../../../../../../shared/defguard-ui/components/Button/Button';
import type { ButtonProps } from '../../../../../../../shared/defguard-ui/components/Button/types';
import { EmptyStateFlexible } from '../../../../../../../shared/defguard-ui/components/EmptyStateFlexible/EmptyStateFlexible';
import { Icon } from '../../../../../../../shared/defguard-ui/components/Icon';
import type {
  MenuItemProps,
  MenuItemsGroup,
} from '../../../../../../../shared/defguard-ui/components/Menu/types';
import { tableEditColumnSize } from '../../../../../../../shared/defguard-ui/components/table/consts';
import { TableBody } from '../../../../../../../shared/defguard-ui/components/table/TableBody/TableBody';
import { TableCell } from '../../../../../../../shared/defguard-ui/components/table/TableCell/TableCell';
import { TableEditCell } from '../../../../../../../shared/defguard-ui/components/table/TableEditCell/TableEditCell';
import { TableFlexCell } from '../../../../../../../shared/defguard-ui/components/table/TableFlexCell/TableFlexCell';
import { TableRowContainer } from '../../../../../../../shared/defguard-ui/components/table/TableRowContainer/TableRowContainer';
import { TableTop } from '../../../../../../../shared/defguard-ui/components/table/TableTop/TableTop';
import { Snackbar } from '../../../../../../../shared/defguard-ui/providers/snackbar/snackbar';
import { TooltipContent } from '../../../../../../../shared/defguard-ui/providers/tooltip/TooltipContent';
import { TooltipProvider } from '../../../../../../../shared/defguard-ui/providers/tooltip/TooltipContext';
import { TooltipTrigger } from '../../../../../../../shared/defguard-ui/providers/tooltip/TooltipTrigger';
import { isPresent } from '../../../../../../../shared/defguard-ui/utils/isPresent';
import { openModal } from '../../../../../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../../../../../shared/hooks/modalControls/modalTypes';
import { useApp } from '../../../../../../../shared/hooks/useApp';
import { useAuth } from '../../../../../../../shared/hooks/useAuth';
import { displayDate } from '../../../../../../../shared/utils/displayDate';
import { useUserProfile } from '../../../../hooks/useUserProfilePage';

interface RowData extends UserDevice {
  connected_at: string | null;
  connected_ip: string | null;
  network_name: string | null;
}

const displayDateOrNull = (value?: string | number | null): string | null => {
  if (isPresent(value)) {
    return displayDate(value);
  }
  return null;
};

export const ProfileDevicesTable = () => {
  const devices = useUserProfile((s) => s.devices);

  const rowData = useMemo((): RowData[] => {
    const rowData: RowData[] = devices.map((device) => {
      const ordered = orderBy(
        device.networks.filter((network) => isPresent(network.last_connected_at)),
        ['last_connected_at'],
        ['desc'],
      );
      const latestConnection = ordered.at(0);

      const row: RowData = {
        ...device,
        connected_at: displayDateOrNull(latestConnection?.last_connected_at),
        connected_ip: latestConnection?.last_connected_ip ?? null,
        network_name: latestConnection?.network_name ?? null,
      };
      return row;
    });
    return rowData;
  }, [devices]);

  if (isPresent(rowData)) return <DevicesTable rowData={rowData} />;

  return null;
};

const columnHelper = createColumnHelper<RowData>();

const DevicesTable = ({ rowData }: { rowData: RowData[] }) => {
  const info = useApp((s) => s.appInfo);
  const isAdmin = useAuth((s) => s.isAdmin);
  const devices = useUserProfile((s) => s.devices);
  const user = useUserProfile((s) => s.user);
  const username = user.username;

  const reservedNames = useMemo(() => rowData.map((row) => row.name), [rowData]);
  const reservedPubkeys = useMemo(
    () => rowData.map((row) => row.wireguard_pubkey),
    [rowData],
  );

  const addDeviceProps = useMemo(
    (): ButtonProps => ({
      text: 'Add device',
      variant: 'primary',
      testId: 'add-device',
      iconLeft: 'add-device',
      disabled: !info.network_present,
      onClick: () => {
        useAddUserDeviceModal.getState().open({
          devices,
          user,
        });
      },
    }),
    [devices, user, info.network_present],
  );

  const makeRowMenu = useCallback(
    (row: RowData): MenuItemsGroup[] => {
      const items: MenuItemProps[] = [
        {
          text: m.controls_edit(),
          icon: 'edit',
          onClick: () => {
            openModal(ModalName.EditUserDevice, {
              device: row,
              reservedNames: reservedNames,
              reservedPubkeys: reservedPubkeys,
              username,
            });
          },
        },
      ];
      if (isAdmin) {
        items.push({
          text: m.profile_devices_menu_ip_settings(),
          icon: 'gateway',
          testId: 'assign-device-ip',
          onClick: () => {
            api.device
              .getDeviceIps(username, row.id)
              .then(({ data: locationData }) => {
                openModal(ModalName.AssignUserDeviceIP, {
                  device: row,
                  username,
                  locationData,
                });
              })
              .catch((error) => {
                Snackbar.error('Failed to load device IP settings');
                console.error(error);
              });
          },
        });
      }
      items.push(
        {
          text: m.profile_devices_menu_show_config(),
          onClick: () => {
            api.device.getDeviceConfigs(row).then((modalData) => {
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
              contentMd: m.modal_delete_user_device_body({ name: row.name }),
              actionPromise: () => api.device.deleteDevice(row.id),
              invalidateKeys: [['user-overview'], ['user', username], ['network']],
              submitProps: { text: m.controls_delete(), variant: 'critical' },
              onSuccess: () => Snackbar.default(m.user_device_delete_success()),
              onError: () => Snackbar.error(m.user_device_delete_failed()),
            });
          },
          variant: 'danger',
          icon: 'delete',
        },
      );
      return [{ items }];
    },
    [reservedNames, username, isAdmin, reservedPubkeys],
  );

  const tableColumns = useMemo(
    () => [
      columnHelper.accessor('name', {
        header: m.profile_devices_col_name(),
        size: 300,
        minSize: 300,
        enableSorting: true,
        meta: {
          flex: true,
        },
        cell: (info) => (
          <TableCell>
            {info.row.original.biometry_enabled && (
              <TooltipProvider>
                <TooltipTrigger>
                  <Icon icon="biometric" />
                </TooltipTrigger>
                <TooltipContent>
                  <p>{m.profile_devices_tooltip_biometric()}</p>
                </TooltipContent>
              </TooltipProvider>
            )}
            <span>{info.getValue()}</span>
          </TableCell>
        ),
      }),
      columnHelper.accessor('connected_ip', {
        id: 'public_ip',
        header: m.profile_devices_col_pub_ip(),
        enableSorting: false,
        minSize: 350,
        cell: (info) => CellWithFallback(info.getValue()),
      }),
      columnHelper.accessor('network_name', {
        id: 'connected_through',
        header: m.profile_devices_col_location(),
        enableSorting: false,
        minSize: 200,
        cell: (info) => CellWithFallback(info.getValue()),
      }),
      columnHelper.accessor('connected_at', {
        id: 'connected_at',
        header: m.profile_devices_col_connected(),
        enableSorting: false,
        minSize: 200,
        cell: (info) => CellWithFallback(info.getValue()),
      }),
      columnHelper.display({
        id: 'edit',
        header: '',
        size: tableEditColumnSize,
        cell: (info) => {
          const menuItems = makeRowMenu(info.row.original);
          return <TableEditCell menuItems={menuItems} />;
        },
      }),
    ],
    [makeRowMenu],
  );

  const renderExpandedRow = useCallback(
    (row: Row<RowData>, isLast = false) => (
      <>
        {row.original.networks.map((network, index) => (
          <TableRowContainer
            className={clsx({
              last: isLast && index === row.original.networks.length - 1,
            })}
            key={network.network_id}
            assignColumnSizing
          >
            <TableCell alignContent="center" noPadding>
              <Icon icon="enter" />
            </TableCell>
            <TableCell className="device-name">
              <Icon icon="location" />
              <span>{network.network_name}</span>
              {isPresent(network.network_gateway_ip) && (
                <Badge variant="neutral" text={network.network_gateway_ip} />
              )}
            </TableCell>
            {CellWithFallback(
              network.device_wireguard_ips.length > 0
                ? network.device_wireguard_ips.join(', ')
                : null,
            )}
            {CellWithFallback(network.last_connected_ip)}
            {CellWithFallback(displayDateOrNull(network.last_connected_at))}
            <TableFlexCell />
          </TableRowContainer>
        ))}
      </>
    ),
    [],
  );

  const expandedRowHeaders = useMemo(
    () => [
      m.profile_devices_col_location_name(),
      m.profile_devices_col_location_ip(),
      m.profile_devices_col_location_connected_from(),
      m.profile_devices_col_location_connected(),
    ],
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
    columns: tableColumns,
    data: rowData,
    getRowId: (row) => String(row.id),
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
    getExpandedRowModel: getExpandedRowModel(),
    getRowCanExpand: (row) => row.original.networks.length > 0,
    enableExpanding: true,
    enableRowSelection: false,
    enableSorting: true,
    columnResizeMode: 'onChange',
  });

  return (
    <>
      {rowData.length === 0 && (
        <EmptyStateFlexible
          title="No devices"
          subtitle="To add new device click the button below."
          primaryAction={addDeviceProps}
        />
      )}
      {rowData.length > 0 && (
        <>
          <TableTop text="All devices">
            <Button {...addDeviceProps} />
          </TableTop>
          <TableBody
            id="profile-devices-table"
            table={table}
            renderExpandedRow={renderExpandedRow}
            expandedHeaders={expandedRowHeaders}
          />
        </>
      )}
    </>
  );
};

const CellWithFallback = (value?: string | null) => {
  const neverConnected = !isPresent(value);
  return (
    <TableCell>
      <span
        className={clsx({
          'never-connected': neverConnected,
        })}
      >
        {value ?? m.profile_devices_col_never_connected()}
      </span>
    </TableCell>
  );
};
