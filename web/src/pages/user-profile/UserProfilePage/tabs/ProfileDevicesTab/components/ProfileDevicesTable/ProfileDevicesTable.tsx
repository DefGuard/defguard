import { useMutation } from '@tanstack/react-query';
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
import { IconButtonMenu } from '../../../../../../../shared/defguard-ui/components/IconButtonMenu/IconButtonMenu';
import type { MenuItemsGroup } from '../../../../../../../shared/defguard-ui/components/Menu/types';
import { tableEditColumnSize } from '../../../../../../../shared/defguard-ui/components/table/consts';
import { TableBody } from '../../../../../../../shared/defguard-ui/components/table/TableBody/TableBody';
import { TableCell } from '../../../../../../../shared/defguard-ui/components/table/TableCell/TableCell';
import { TableFlexCell } from '../../../../../../../shared/defguard-ui/components/table/TableFlexCell/TableFlexCell';
import { TableRowContainer } from '../../../../../../../shared/defguard-ui/components/table/TableRowContainer/TableRowContainer';
import { TableTop } from '../../../../../../../shared/defguard-ui/components/table/TableTop/TableTop';
import { Snackbar } from '../../../../../../../shared/defguard-ui/providers/snackbar/snackbar';
import { isPresent } from '../../../../../../../shared/defguard-ui/utils/isPresent';
import { openModal } from '../../../../../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../../../../../shared/hooks/modalControls/modalTypes';
import { useApp } from '../../../../../../../shared/hooks/useApp';
import { displayDate } from '../../../../../../../shared/utils/displayDate';
import { useUserProfile } from '../../../../hooks/useUserProfilePage';

interface RowData extends UserDevice {
  connected_at: string;
  connected_ip: string;
  network_name: string;
}

export const ProfileDevicesTable = () => {
  const devices = useUserProfile((s) => s.devices);

  const rowData = useMemo((): RowData[] => {
    const rowData: RowData[] = devices.map((device) => {
      const ordered = orderBy(
        device.networks.filter((network) => isPresent(network.last_connected_at)),
        ['last_connected_at'],
        ['desc'],
      );
      const fallbackValue = m.profile_devices_col_never_connected();
      const latestConnection = ordered.at(0);

      const row: RowData = {
        ...device,
        connected_at: latestConnection?.last_connected_at
          ? displayDate(latestConnection.last_connected_at)
          : fallbackValue,
        connected_ip: latestConnection?.last_connected_ip ?? fallbackValue,
        network_name: latestConnection?.network_name ?? fallbackValue,
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
  const devices = useUserProfile((s) => s.devices);
  const user = useUserProfile((s) => s.user);
  const username = user.username;

  const reservedNames = useMemo(() => rowData.map((row) => row.name), [rowData]);

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

  const { mutate: deleteDevice } = useMutation({
    mutationFn: api.device.deleteDevice,
    meta: {
      invalidate: [['user-overview'], ['user', username]],
    },
  });

  const makeRowMenu = useCallback(
    (row: RowData): MenuItemsGroup[] => [
      {
        items: [
          {
            text: m.controls_edit(),
            icon: 'edit',
            onClick: () => {
              openModal(ModalName.EditUserDevice, {
                device: row,
                reservedNames: reservedNames,
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
          },
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
              deleteDevice(row.id);
            },
            variant: 'danger',
            icon: 'delete',
          },
        ],
      },
    ],
    [reservedNames, username, deleteDevice],
  );

  const tableColumns = useMemo(
    () => [
      columnHelper.accessor('name', {
        header: m.profile_devices_col_name(),
        cell: (info) => (
          <TableCell>
            {info.row.original.biometry_enabled && <Icon icon="biometric" />}
            <span>{info.getValue()}</span>
          </TableCell>
        ),
        enableSorting: true,
        meta: {
          flex: true,
        },
      }),
      columnHelper.accessor('connected_ip', {
        id: 'public_ip',
        header: m.profile_devices_col_pub_ip(),
        enableSorting: false,
        size: 350,
        cell: (info) => (
          <TableCell>
            <span>{info.getValue() ?? m.profile_devices_col_never_connected()}</span>
          </TableCell>
        ),
      }),
      columnHelper.accessor('network_name', {
        id: 'connected_through',
        header: m.profile_devices_col_location(),
        enableSorting: false,
        size: 175,
        cell: (info) => (
          <TableCell>
            <span>{info.getValue() ?? m.profile_devices_col_never_connected()}</span>
          </TableCell>
        ),
      }),
      columnHelper.accessor('connected_at', {
        id: 'connected_at',
        header: m.profile_devices_col_connected(),
        enableSorting: false,
        size: 175,
        cell: (info) => {
          return (
            <TableCell>
              <span>{info.getValue()}</span>
            </TableCell>
          );
        },
      }),
      columnHelper.display({
        id: 'edit',
        header: '',
        size: tableEditColumnSize,
        cell: (info) => {
          const menuItems = makeRowMenu(info.row.original);
          return (
            <TableCell alignContent="center" noPadding>
              <IconButtonMenu icon="menu" menuItems={menuItems} />
            </TableCell>
          );
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
            <TableCell>
              <span>
                {network.device_wireguard_ips.join(', ') ??
                  m.profile_devices_col_never_connected()}
              </span>
            </TableCell>
            <TableCell>
              <span>
                {network.last_connected_ip ?? m.profile_devices_col_never_connected()}
              </span>
            </TableCell>
            <TableCell>
              <span>
                {!network.last_connected_at && m.profile_devices_col_never_connected()}
                {isPresent(network.last_connected_at) &&
                  displayDate(network.last_connected_at)}
              </span>
            </TableCell>
            <TableCell empty />
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
      '',
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
            table={table}
            renderExpandedRow={renderExpandedRow}
            expandedHeaders={expandedRowHeaders}
          />
        </>
      )}
    </>
  );
};
