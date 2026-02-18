import { useInfiniteQuery } from '@tanstack/react-query';
import { useParams, useSearch } from '@tanstack/react-router';
import {
  createColumnHelper,
  getCoreRowModel,
  type SortingState,
  useReactTable,
} from '@tanstack/react-table';
import { useMemo, useState } from 'react';
import api from '../../shared/api/api';
import type { LocationConnectedNetworkDevice } from '../../shared/api/types';
import { TableSkeleton } from '../../shared/components/skeleton/TableSkeleton/TableSkeleton';
import { TableValuesListCell } from '../../shared/components/TableValuesListCell/TableValuesListCell';
import { EmptyStateFlexible } from '../../shared/defguard-ui/components/EmptyStateFlexible/EmptyStateFlexible';
import { TableBody } from '../../shared/defguard-ui/components/table/TableBody/TableBody';
import { TableCell } from '../../shared/defguard-ui/components/table/TableCell/TableCell';
import { ConnectionDurationCell } from './components/ConnectionDurationCell';
import { DeviceTrafficChartCell } from './components/DeviceTrafficChartCell/DeviceTrafficChartCell';

const columnHelper = createColumnHelper<LocationConnectedNetworkDevice>();

export const LocationOverviewNetworkDevicesTable = () => {
  const search = useSearch({ from: '/_authorized/_default/vpn-overview/$locationId' });
  const { locationId } = useParams({
    from: '/_authorized/_default/vpn-overview/$locationId',
  });

  const { data, fetchNextPage, isFetchingNextPage, isLoading } = useInfiniteQuery({
    queryKey: [
      'network',
      Number(locationId),
      'stats',
      'connected_network_devices',
      search.period,
    ],
    initialPageParam: 1,
    queryFn: ({ pageParam }) =>
      api.location.getLocationConnectedNetworkDevices({
        id: Number(locationId),
        from: search.period,
        page: pageParam,
      }),
    getNextPageParam: (lastPage) => lastPage?.pagination.next_page,
    getPreviousPageParam: (page) => {
      if (page.pagination.current_page !== 1) {
        return page.pagination.current_page - 1;
      }
      return null;
    },
  });

  const flatQueryData = useMemo(() => data?.pages.flat() ?? null, [data?.pages]);
  const flatData = useMemo(
    () => flatQueryData?.flatMap((page) => page.data) ?? [],
    [flatQueryData],
  );

  const lastItem = flatQueryData ? flatQueryData[flatQueryData?.length - 1] : null;
  const pagination = lastItem ? lastItem.pagination : null;

  const [sortState, setSortState] = useState<SortingState>([
    {
      id: 'device_name',
      desc: false,
    },
  ]);

  const columns = useMemo(
    () => [
      columnHelper.accessor('device_name', {
        header: 'Device name',
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
        size: 200,
        header: 'Public IP',
        cell: (info) => (
          <TableCell>
            <span>{info.getValue()}</span>
          </TableCell>
        ),
      }),
      columnHelper.accessor('vpn_ips', {
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
          const { stats, total_download, total_upload } = row;
          return (
            <DeviceTrafficChartCell
              stats={stats}
              download={total_download}
              upload={total_upload}
            />
          );
        },
      }),
    ],
    [],
  );

  const table = useReactTable({
    state: {
      sorting: sortState,
    },
    columns,
    data: flatData,
    getCoreRowModel: getCoreRowModel(),
    onSortingChange: setSortState,
    manualSorting: true,
    enableSorting: true,
    enableExpanding: false,
    enableRowSelection: false,
    columnResizeMode: 'onChange',
  });

  if (isLoading) return <TableSkeleton />;

  if (flatData.length === 0)
    return (
      <EmptyStateFlexible
        title="No connected network devices"
        subtitle="Wait for some device to connect"
      />
    );

  return (
    <TableBody
      table={table}
      loadingNextPage={isFetchingNextPage}
      onNextPage={() => {
        fetchNextPage();
      }}
      hasNextPage={pagination?.next_page !== null}
    />
  );
};
