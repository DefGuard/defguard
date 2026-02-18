import { useInfiniteQuery } from '@tanstack/react-query';
import { useParams, useSearch } from '@tanstack/react-router';
import {
  createColumnHelper,
  getCoreRowModel,
  getExpandedRowModel,
  type SortingState,
  useReactTable,
} from '@tanstack/react-table';
import { useMemo, useState } from 'react';
import api from '../../shared/api/api';
import type { LocationConnectedUser } from '../../shared/api/types';
import { TableSkeleton } from '../../shared/components/skeleton/TableSkeleton/TableSkeleton';
import { TableValuesListCell } from '../../shared/components/TableValuesListCell/TableValuesListCell';
import { Avatar } from '../../shared/defguard-ui/components/Avatar/Avatar';
import { EmptyStateFlexible } from '../../shared/defguard-ui/components/EmptyStateFlexible/EmptyStateFlexible';
import { Icon } from '../../shared/defguard-ui/components/Icon';
import { TableBody } from '../../shared/defguard-ui/components/table/TableBody/TableBody';
import { TableCell } from '../../shared/defguard-ui/components/table/TableCell/TableCell';
import { ConnectionDurationCell } from './components/ConnectionDurationCell';
import { DeviceTrafficChartCell } from './components/DeviceTrafficChartCell/DeviceTrafficChartCell';

const columnHelper = createColumnHelper<LocationConnectedUser>();

const expansionHeaders = [
  'Device name',
  'Public IP',
  'VPN IP',
  'Connected',
  '',
  'Device traffic',
];

export const LocationOverviewUsersTable = () => {
  const search = useSearch({ from: '/_authorized/_default/vpn-overview/$locationId' });
  const { locationId } = useParams({
    from: '/_authorized/_default/vpn-overview/$locationId',
  });

  const { data, fetchNextPage, isFetchingNextPage, isLoading } = useInfiniteQuery({
    queryKey: ['network', Number(locationId), 'stats', 'connected_users'],
    initialPageParam: 1,
    queryFn: ({ pageParam }) =>
      api.location.getLocationConnectedUsers({
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
      id: 'name',
      desc: false,
    },
  ]);

  const columns = useMemo(
    () => [
      columnHelper.accessor('full_name', {
        header: 'User name',
        meta: {
          flex: true,
        },
        enableSorting: true,
        cell: (info) => (
          <TableCell>
            <Avatar
              variant="initials"
              firstName={info.row.original.first_name}
              lastName={info.row.original.last_name}
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
      columnHelper.accessor('vpn_ips', {
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
            <span>{info.row.original.connected_devices_count}</span>
          </TableCell>
        ),
      }),
      columnHelper.display({
        id: 'stats',
        size: 500,
        header: 'Traffic',
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

  // const renderExpansionRow = useCallback(
  //   (row: Row<RowData>, isLast = false) =>
  //     row.original.devices.map((device, expandIndex) => (
  //       <TableRowContainer
  //         className={clsx({
  //           last: isLast && expandIndex === row.original.devices.length - 1,
  //         })}
  //         key={device.id}
  //       >
  //         <TableCell alignContent="center" noPadding>
  //           <Icon icon="enter" />
  //         </TableCell>
  //         <TableCell>
  //           <Icon icon="devices" staticColor={ThemeVariable.FgSuccess} />
  //           <span>{device.name}</span>
  //         </TableCell>
  //         <TableCell>
  //           <span>{device.public_ip}</span>
  //         </TableCell>
  //         <TableValuesListCell values={device.wireguard_ips} />
  //         <ConnectionDurationCell connectedAt={device.connected_at} />
  //         <TableCell empty />
  //         <DeviceTrafficChartCell
  //           upload={device.upload}
  //           download={device.download}
  //           traffic={device.stats}
  //         />
  //       </TableRowContainer>
  //     )),
  //   [],
  // );

  const table = useReactTable({
    state: {
      sorting: sortState,
    },
    columns,
    data: flatData,
    getExpandedRowModel: getExpandedRowModel(),
    getCoreRowModel: getCoreRowModel(),
    // getRowCanExpand: (row) => row.original.devices?.length >= 1,
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
        title="No connected users"
        subtitle="Wait for some user to connect"
      />
    );

  return (
    <TableBody
      table={table}
      expandedHeaders={expansionHeaders}
      // renderExpandedRow={renderExpansionRow}
      loadingNextPage={isFetchingNextPage}
      onNextPage={() => {
        fetchNextPage();
      }}
      hasNextPage={pagination?.next_page !== null}
    />
  );
};
