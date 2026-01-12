import {
  createColumnHelper,
  getCoreRowModel,
  useReactTable,
} from '@tanstack/react-table';
import { useMemo } from 'react';
import { activityLogEventDisplay } from '../../shared/api/activity-log-types';
import type {
  ActivityLogEvent,
  ActivityLogFilters,
  PaginationMeta,
} from '../../shared/api/types';
import { EmptyStateFlexible } from '../../shared/defguard-ui/components/EmptyStateFlexible/EmptyStateFlexible';
import { TableBody } from '../../shared/defguard-ui/components/table/TableBody/TableBody';
import { TableCell } from '../../shared/defguard-ui/components/table/TableCell/TableCell';
import { TableTop } from '../../shared/defguard-ui/components/table/TableTop/TableTop';
import { useApiToTableState } from '../../shared/defguard-ui/hooks/useApiToTableState';
import { isPresent } from '../../shared/defguard-ui/utils/isPresent';
import { displayDate } from '../../shared/utils/displayDate';

type RowData = ActivityLogEvent;

const columnHelper = createColumnHelper<RowData>();

interface Props {
  data: RowData[];
  filters: Partial<ActivityLogFilters>;
  pagination: PaginationMeta;
  hasNextPage: boolean;
  loadingNextPage: boolean;
  onNextPage: () => void;
}

export const ActivityLogTable = ({
  data,
  filters,
  pagination,
  loadingNextPage,
  hasNextPage,
  onNextPage,
}: Props) => {
  const { sortingState } = useApiToTableState<RowData>({
    ...filters,
    ...pagination,
    defaultSortingKey: 'timestamp',
  });

  const columns = useMemo(
    () => [
      columnHelper.accessor('timestamp', {
        header: 'Date',
        enableSorting: true,
        minSize: 160,
        cell: (info) => {
          const data = info.getValue();
          const formatted = displayDate(data);
          return (
            <TableCell>
              <span>{formatted}</span>
            </TableCell>
          );
        },
      }),
      columnHelper.accessor('username', {
        header: 'User',
        minSize: 150,
        cell: (info) => (
          <TableCell>
            <span>{info.getValue()}</span>
          </TableCell>
        ),
      }),
      columnHelper.accessor('ip', {
        header: 'IP',
        minSize: 150,
        cell: (info) => (
          <TableCell>
            <span>{info.getValue()}</span>
          </TableCell>
        ),
      }),
      columnHelper.accessor('location', {
        header: 'Location',
        minSize: 130,
        cell: (info) => {
          const value = info.getValue();
          return (
            <TableCell>
              {isPresent(value) ? <span>{value}</span> : <span>{`~`}</span>}
            </TableCell>
          );
        },
      }),
      columnHelper.accessor('event', {
        header: 'Event',
        minSize: 190,
        cell: (info) => {
          const event = info.getValue();
          return (
            <TableCell>
              <span>{activityLogEventDisplay[event]}</span>
            </TableCell>
          );
        },
      }),
      columnHelper.accessor('module', {
        header: 'Module',
        minSize: 140,
        cell: (info) => {
          const value = info.getValue();
          return (
            <TableCell>
              <span>{value}</span>
            </TableCell>
          );
        },
      }),
      columnHelper.accessor('description', {
        header: 'Description',
        minSize: 300,
        cell: (info) => {
          const value = info.getValue();
          return (
            <TableCell>
              <span>{value}</span>
            </TableCell>
          );
        },
      }),
    ],
    [],
  );

  const table = useReactTable({
    state: {
      sorting: sortingState,
    },
    data,
    columns,
    columnResizeMode: 'onChange',
    getCoreRowModel: getCoreRowModel(),
    enableRowSelection: false,
    enableExpanding: false,
    enableSorting: true,
  });

  if (data.length === 0)
    return (
      <EmptyStateFlexible
        icon="log"
        title={`You don't have any logs.`}
        subtitle={`Activity logs will be displayed here once events occur.`}
      />
    );

  return (
    <>
      <TableTop text="Activity"></TableTop>
      <TableBody
        table={table}
        hasNextPage={hasNextPage}
        onNextPage={onNextPage}
        loadingNextPage={loadingNextPage}
      />
    </>
  );
};
