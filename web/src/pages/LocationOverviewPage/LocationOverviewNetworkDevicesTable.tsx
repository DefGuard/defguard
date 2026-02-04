import {
  createColumnHelper,
  getCoreRowModel,
  getSortedRowModel,
  useReactTable,
} from '@tanstack/react-table';
import { sumBy } from 'lodash-es';
import { useMemo } from 'react';
import type { DeviceStats, LocationDevicesStats } from '../../shared/api/types';
import { TableValuesListCell } from '../../shared/components/TableValuesListCell/TableValuesListCell';
import { EmptyStateFlexible } from '../../shared/defguard-ui/components/EmptyStateFlexible/EmptyStateFlexible';
import { tableActionColumnSize } from '../../shared/defguard-ui/components/table/consts';
import { TableBody } from '../../shared/defguard-ui/components/table/TableBody/TableBody';
import { TableCell } from '../../shared/defguard-ui/components/table/TableCell/TableCell';
import { mapTransferToChart, type TransferChartData } from '../../shared/utils/stats';
import { ConnectionDurationCell } from './components/ConnectionDurationCell';
import { DeviceTrafficChartCell } from './components/DeviceTrafficChartCell/DeviceTrafficChartCell';

type RowData = Omit<DeviceStats, 'stats'> & {
  stats: TransferChartData[];
  upload: number;
  download: number;
};

const columnHelper = createColumnHelper<RowData>();

export const LocationOverviewNetworkDevicesTable = ({
  data,
}: {
  data: LocationDevicesStats['network_devices'];
}) => {
  const mappedData = useMemo((): RowData[] => {
    const res: RowData[] = data.map((device) => ({
      ...device,
      stats: mapTransferToChart(device.stats),
      upload: sumBy(device.stats, (s) => s.upload),
      download: sumBy(device.stats, (s) => s.download),
    }));
    return res;
  }, [data]);

  const columns = useMemo(
    () => [
      columnHelper.display({
        id: 'empty',
        header: '',
        size: tableActionColumnSize,
        cell: () => <TableCell empty />,
      }),
      columnHelper.accessor('name', {
        header: 'Device name',
        sortingFn: 'text',
        enableSorting: true,
        meta: {
          flex: true,
        },
        cell: (info) => (
          <TableCell>
            <span>{info.getValue()}</span>
          </TableCell>
        ),
      }),
      columnHelper.accessor('public_ip', {
        size: 200,
        header: 'Public IP',
        cell: (info) => (
          <TableCell>
            <span>{info.getValue()}</span>
          </TableCell>
        ),
      }),
      columnHelper.accessor('wireguard_ips', {
        size: 250,
        header: 'VPN IP',
        cell: (info) => <TableValuesListCell values={info.getValue()} />,
      }),
      columnHelper.accessor('connected_at', {
        size: 125,
        header: 'Connected',
        cell: (info) => <ConnectionDurationCell connectedAt={info.getValue()} />,
      }),
      columnHelper.display({
        id: 'stats',
        header: 'Traffic',
        size: 500,
        cell: (info) => {
          const row = info.row.original;
          return (
            <DeviceTrafficChartCell
              traffic={row.stats}
              download={row.download}
              upload={row.upload}
            />
          );
        },
      }),
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
    columns,
    data: mappedData,
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
    enableExpanding: false,
    enableSorting: true,
    enableRowSelection: false,
  });

  if (data.length === 0)
    return (
      <EmptyStateFlexible
        title="No connected network devices"
        subtitle="Wait for some device to connect"
      />
    );

  return <TableBody table={table} />;
};
