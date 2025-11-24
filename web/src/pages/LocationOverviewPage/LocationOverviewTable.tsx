import {
  createColumnHelper,
  getCoreRowModel,
  getExpandedRowModel,
  type SortingState,
  useReactTable,
} from '@tanstack/react-table';
import dayjs from 'dayjs';
import { orderBy } from 'lodash-es';
import { useMemo, useState } from 'react';
import type { DeviceStats, LocationUserDeviceStats } from '../../shared/api/types';
import { TableValuesListCell } from '../../shared/components/TableValuesListCell/TableValuesListCell';
import { EmptyStateFlexible } from '../../shared/defguard-ui/components/EmptyStateFlexible/EmptyStateFlexible';
import { Icon } from '../../shared/defguard-ui/components/Icon';
import { TableBody } from '../../shared/defguard-ui/components/table/TableBody/TableBody';
import { TableCell } from '../../shared/defguard-ui/components/table/TableCell/TableCell';
import { TableTop } from '../../shared/defguard-ui/components/table/TableTop/TableTop';

type RowData = {
  devices: DeviceStats[];
} & DeviceStats;

type Props = {
  data: LocationUserDeviceStats[];
};

const columnHelper = createColumnHelper<RowData>();

export const LocationOverviewUsersTable = ({ data }: Props) => {
  const mapped = useMemo(
    () =>
      data.map(({ user, devices }): RowData => {
        const oldest = orderBy(devices, (d) => d.connected_at, ['asc'])[0];
        return {
          ...oldest,
          id: user.id,
          devices: devices,
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
            <span>{info.getValue()}</span>
          </TableCell>
        ),
      }),
      columnHelper.accessor('public_ip', {
        header: 'Public IP',
        cell: (info) => (
          <TableCell>
            <span>{info.getValue()}</span>
          </TableCell>
        ),
      }),
      columnHelper.accessor('wireguard_ips', {
        header: 'VPN IP',
        cell: (info) => <TableValuesListCell values={info.getValue()} />,
      }),
      columnHelper.accessor('connected_at', {
        header: 'Connected',
        cell: (info) => {
          const now = dayjs();
          const from = dayjs.utc(info.getValue()).local();
          const diff = now.diff(from);
          return (
            <TableCell>
              <span>{`${diff / 1000}m`}</span>
            </TableCell>
          );
        },
      }),
      columnHelper.display({
        id: 'devices_count',
        header: 'Device',
        cell: (info) => (
          <TableCell>
            <Icon icon="connected-devices" />
            <span>{info.row.original.devices.length}</span>
          </TableCell>
        ),
      }),
      columnHelper.accessor('stats', {
        header: 'Traffic',
        cell: (info) => (
          <TableCell>
            <span>{info.getValue().length}</span>
          </TableCell>
        ),
      }),
    ],
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
  });

  if (data.length === 0)
    return (
      <EmptyStateFlexible
        title="No connected users"
        subtitle="Wait for some user to connect"
      />
    );

  return (
    <>
      <TableTop text="Connected User's Devices"></TableTop>
      <TableBody table={table} />
    </>
  );
};
