import {
  createColumnHelper,
  getCoreRowModel,
  getExpandedRowModel,
  type Row,
  type SortingState,
  useReactTable,
} from '@tanstack/react-table';
import clsx from 'clsx';
import { orderBy, sumBy } from 'lodash-es';
import { type CSSProperties, useCallback, useMemo, useState } from 'react';
import type { DeviceStats, LocationUserDeviceStats } from '../../shared/api/types';
import { TableValuesListCell } from '../../shared/components/TableValuesListCell/TableValuesListCell';
import { Avatar } from '../../shared/defguard-ui/components/Avatar/Avatar';
import { EmptyStateFlexible } from '../../shared/defguard-ui/components/EmptyStateFlexible/EmptyStateFlexible';
import { Icon } from '../../shared/defguard-ui/components/Icon';
import { TableBody } from '../../shared/defguard-ui/components/table/TableBody/TableBody';
import { TableCell } from '../../shared/defguard-ui/components/table/TableCell/TableCell';
import { TableRowContainer } from '../../shared/defguard-ui/components/table/TableRowContainer/TableRowContainer';
import { ThemeVariable } from '../../shared/defguard-ui/types';
import { mapTransferToChart, type TransferChartData } from '../../shared/utils/stats';
import { ConnectionDurationCell } from './components/ConnectionDurationCell';
import { DeviceTrafficChartCell } from './components/DeviceTrafficChartCell/DeviceTrafficChartCell';
import { overviewTableUtils } from './utils/overviewTableUtils';

type TableDevice = Omit<DeviceStats, 'stats'> & {
  stats: TransferChartData[];
  upload: number;
  download: number;
};

type RowData = {
  firstName: string;
  lastName: string;
  devices: TableDevice[];
} & TableDevice;

type Props = {
  data: LocationUserDeviceStats[];
};

const columnHelper = createColumnHelper<RowData>();

const expansionHeaders = [
  'Device name',
  'Public IP',
  'VPN IP',
  'Connected',
  '',
  'Device traffic',
];

export const LocationOverviewUsersTable = ({ data }: Props) => {
  const mapped = useMemo(
    () =>
      data.map(({ user, devices }): RowData => {
        const oldest = orderBy(devices, (d) => d.connected_at, ['asc'])[0];
        const formattedDevices = devices.map((d) => ({
          ...d,
          stats: mapTransferToChart(d.stats),
          download: sumBy(d.stats, (s) => s.download),
          upload: sumBy(d.stats, (s) => s.upload),
        }));

        const mergedStats = overviewTableUtils.mergeStats(devices);

        return {
          id: user.id,
          devices: formattedDevices,
          name: `${user.first_name} ${user.last_name}`,
          firstName: user.first_name,
          lastName: user.last_name,
          stats: mergedStats,
          download: sumBy(mergedStats, (s) => s.download),
          upload: sumBy(mergedStats, (s) => s.upload),
          connected_at: oldest.connected_at,
          public_ip: oldest.public_ip,
          wireguard_ips: oldest.wireguard_ips,
        };
      }),
    [data],
  );

  const [sortState, setSortState] = useState<SortingState>([
    {
      id: 'name',
      desc: false,
    },
  ]);

  const transformedData = useMemo(() => {
    let res = mapped;
    const sorting = sortState[0];
    // apply sorting
    if (sorting) {
      const { id, desc } = sorting;
      const direction = desc ? 'desc' : 'asc';
      res = orderBy(
        res.map((row) => ({ ...row, devices: orderBy(row.devices, [id], [direction]) })),
        [id],
        [direction],
      );
    }
    return res;
  }, [mapped, sortState[0]]);

  const columns = useMemo(
    () => [
      columnHelper.accessor('name', {
        header: 'User name',
        meta: {
          flex: true,
        },
        enableSorting: true,
        cell: (info) => (
          <TableCell>
            <Avatar
              variant="initials"
              firstName={info.row.original.firstName}
              lastName={info.row.original.lastName}
            />
            <span>{info.getValue()}</span>
          </TableCell>
        ),
      }),
      columnHelper.accessor('public_ip', {
        header: 'Public IP',
        size: 200,
        cell: (info) => (
          <TableCell>
            <span>{info.getValue()}</span>
          </TableCell>
        ),
      }),
      columnHelper.accessor('wireguard_ips', {
        header: 'VPN IP',
        size: 250,
        cell: (info) => <TableValuesListCell values={info.getValue()} />,
      }),
      columnHelper.accessor('connected_at', {
        size: 125,
        header: 'Connected',
        cell: (info) => <ConnectionDurationCell connectedAt={info.getValue()} />,
      }),
      columnHelper.display({
        size: 125,
        id: 'devices_count',
        header: 'Device',
        cell: (info) => (
          <TableCell className="devices-count-cell">
            <Icon icon="connected-devices" />
            <span>{info.row.original.devices.length}</span>
          </TableCell>
        ),
      }),
      columnHelper.display({
        id: 'stats',
        size: 500,
        header: 'Traffic',
        cell: (info) => {
          const row = info.row.original;
          const { stats, download, upload } = row;
          return (
            <DeviceTrafficChartCell traffic={stats} download={download} upload={upload} />
          );
        },
      }),
    ],
    [],
  );

  const renderExpansionRow = useCallback(
    (row: Row<RowData>, rowStyles: CSSProperties, isLast = false) =>
      row.original.devices.map((device) => (
        <TableRowContainer
          className={clsx({
            last: isLast,
          })}
          key={device.id}
          style={rowStyles}
        >
          <TableCell alignContent="center" noPadding>
            <Icon icon="enter" />
          </TableCell>
          <TableCell>
            <Icon icon="devices" staticColor={ThemeVariable.FgSuccess} />
            <span>{device.name}</span>
          </TableCell>
          <TableCell>
            <span>{device.public_ip}</span>
          </TableCell>
          <TableValuesListCell values={device.wireguard_ips} />
          <ConnectionDurationCell connectedAt={device.connected_at} />
          <TableCell empty />
          <DeviceTrafficChartCell
            upload={device.upload}
            download={device.download}
            traffic={device.stats}
          />
        </TableRowContainer>
      )),
    [],
  );

  const table = useReactTable({
    state: {
      sorting: sortState,
    },
    columns,
    data: transformedData,
    getExpandedRowModel: getExpandedRowModel(),
    getCoreRowModel: getCoreRowModel(),
    getRowCanExpand: (row) => row.original.devices?.length >= 1,
    onSortingChange: setSortState,
    manualSorting: true,
    enableSorting: true,
    enableExpanding: true,
    enableRowSelection: false,
  });

  if (data.length === 0)
    return (
      <EmptyStateFlexible
        title="No connected users"
        subtitle="Wait for some user to connect"
      />
    );

  return (
    <TableBody
      table={table}
      expandedHeaders={expansionHeaders}
      renderExpandedRow={renderExpansionRow}
    />
  );
};
