import { useMutation } from '@tanstack/react-query';
import {
  createColumnHelper,
  getCoreRowModel,
  getExpandedRowModel,
  type Row,
  type SortingState,
  useReactTable,
} from '@tanstack/react-table';
import clsx from 'clsx';
import orderBy from 'lodash-es/orderBy';
import { type CSSProperties, useCallback, useMemo, useState } from 'react';
import { m } from '../../../../../../../paraglide/messages';
import api from '../../../../../../../shared/api/api';
import type {
  DeviceNetworkInfo,
  UserDevice,
} from '../../../../../../../shared/api/types';
import { Icon } from '../../../../../../../shared/defguard-ui/components/Icon';
import { IconButtonMenu } from '../../../../../../../shared/defguard-ui/components/IconButtonMenu/IconButtonMenu';
import type { MenuItemsGroup } from '../../../../../../../shared/defguard-ui/components/Menu/types';
import {
  tableActionColumnSize,
  tableEditColumnSize,
} from '../../../../../../../shared/defguard-ui/components/table/consts';
import { TableBody } from '../../../../../../../shared/defguard-ui/components/table/TableBody/TableBody';
import { TableCell } from '../../../../../../../shared/defguard-ui/components/table/TableCell/TableCell';
import { TableExpandCell } from '../../../../../../../shared/defguard-ui/components/table/TableExpandCell/TableExpandCell';
import { TableRowContainer } from '../../../../../../../shared/defguard-ui/components/table/TableRowContainer/TableRowContainer';
import { renderTableCellValue } from '../../../../../../../shared/defguard-ui/components/table/utils/renderTableCellValue';
import { isPresent } from '../../../../../../../shared/defguard-ui/utils/isPresent';
import { openModal } from '../../../../../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../../../../../shared/hooks/modalControls/modalTypes';
import { displayDate } from '../../../../../../../shared/utils/displayDate';
import { useUserProfile } from '../../../../hooks/useUserProfilePage';

interface RowData extends UserDevice {
  connected_at: string;
  connected_ip: string;
  network_gateway_ip: string;
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
        network_gateway_ip: latestConnection?.network_gateway_ip ?? fallbackValue,
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
  const username = useUserProfile((s) => s.user.username);
  const tableData = useMemo(() => rowData, [rowData]);

  const reservedNames = useMemo(() => rowData.map((row) => row.name), [rowData]);

  const { mutate: deleteDevice } = useMutation({
    mutationFn: api.device.deleteDevice,
    meta: {
      invalidate: ['user', username],
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
      columnHelper.display({
        id: 'expand',
        header: '',
        size: tableActionColumnSize,
        cell: (info) => <TableExpandCell row={info.row} />,
      }),
      columnHelper.accessor('name', {
        header: m.profile_devices_col_name(),
        cell: renderTableCellValue,
        enableSorting: true,
        meta: {
          flex: true,
        },
      }),
      columnHelper.accessor('connected_ip', {
        id: 'public_ip',
        header: m.profile_devices_col_pub_ip(),
        enableSorting: false,
        cell: (info) => (
          <TableCell>
            <span>{info.getValue() ?? m.profile_devices_col_never_connected()}</span>
          </TableCell>
        ),
      }),
      columnHelper.accessor('network_gateway_ip', {
        id: 'connected_through',
        header: m.profile_devices_col_location(),
        enableSorting: false,
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
    (row: Row<RowData>, rowStyles: CSSProperties, isLast = false) => (
      <>
        {row.original.networks.map((network) => (
          <TableRowContainer
            className={clsx({
              last: isLast,
            })}
            key={network.network_id}
            style={rowStyles}
          >
            <TableCell alignContent="center" noPadding>
              <Icon icon="enter" />
            </TableCell>
            <TableCell>
              <span>{network.network_name}</span>
            </TableCell>
            <TableCell>
              <span>
                {network.last_connected_ip ?? m.profile_devices_col_never_connected()}
              </span>
            </TableCell>
            <TableCell>
              <span>{network.network_gateway_ip}</span>
            </TableCell>
            <TableCell>
              <span>
                {!network.last_connected_at && m.profile_devices_col_never_connected()}
                {isPresent(network.last_connected_at) &&
                  displayDate(network.last_connected_at)}
              </span>
            </TableCell>
            <TableCell empty />
          </TableRowContainer>
        ))}
      </>
    ),
    [],
  );

  const expandedRowHeaders = useMemo(
    () => [
      '',
      m.profile_devices_col_location_name(),
      m.profile_devices_col_location_ip(),
      m.profile_devices_col_location_connected_from(),
      m.profile_devices_col_location_connected(),
      '',
    ],
    [],
  );

  const [sorting, setSorting] = useState<SortingState>([
    {
      id: 'name',
      desc: false,
    },
  ]);

  const transformedData = useMemo(() => {
    if (!sorting.length) return tableData;

    const sortingValue = sorting[0];

    const sortId = sortingValue.id as 'name';
    const sortDirection = sortingValue.desc ? 'desc' : 'asc';

    const networksSortKey: keyof DeviceNetworkInfo = 'network_name';

    return orderBy(tableData, (obj) => obj[sortId].toLowerCase().replaceAll(' ', ''), [
      sortDirection,
    ]).map((device) => ({
      ...device,
      networks: orderBy(
        device.networks,
        (obj) => obj[networksSortKey].toLowerCase().replaceAll(' ', ''),
        [sortDirection],
      ),
    }));
  }, [tableData, sorting]);

  const table = useReactTable({
    columns: tableColumns,
    data: transformedData,
    state: {
      sorting,
    },
    manualSorting: true,
    getCoreRowModel: getCoreRowModel(),
    getExpandedRowModel: getExpandedRowModel(),
    getRowCanExpand: (row) => row.original.networks.length > 0,
    onSortingChange: setSorting,
  });

  return (
    <TableBody
      table={table}
      renderExpandedRow={renderExpandedRow}
      expandedHeaders={expandedRowHeaders}
    />
  );
};
