import {
  createColumnHelper,
  getCoreRowModel,
  useReactTable,
} from '@tanstack/react-table';
import { useMemo } from 'react';
import { m } from '../../paraglide/messages';
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
import { formatIpForDisplay } from '../../shared/utils/formatIpForDisplay';

type RowData = ActivityLogEvent;

const columnHelper = createColumnHelper<RowData>();
const missingValuePlaceholder = '—';
const activityLogTimestampFormat = 'DD/MM/YYYY | HH:mm:ss';

const renderOptionalTableValue = (
  value: string | null | undefined,
  missingValueLabel: string,
) => {
  if (!isPresent(value)) {
    return <span aria-label={missingValueLabel}>{missingValuePlaceholder}</span>;
  }

  return <span>{value}</span>;
};

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
        header: m.activity_log_col_date(),
        enableSorting: true,
        minSize: 180,
        cell: (info) => {
          const data = info.getValue();
          const formatted = displayDate(data, activityLogTimestampFormat);
          return (
            <TableCell>
              <span>{formatted}</span>
            </TableCell>
          );
        },
      }),
      columnHelper.accessor('username', {
        header: m.activity_log_col_user(),
        minSize: 150,
        cell: (info) => (
          <TableCell>
            <span>{info.getValue()}</span>
          </TableCell>
        ),
      }),
      columnHelper.accessor('ip', {
        header: m.activity_log_col_ip(),
        minSize: 150,
        cell: (info) => {
          const value = info.getValue();
          const displayValue = isPresent(value) ? formatIpForDisplay(value) : value;
          return (
            <TableCell>
              {renderOptionalTableValue(displayValue, m.activity_log_missing_ip())}
            </TableCell>
          );
        },
      }),
      columnHelper.accessor('location', {
        header: m.activity_log_col_location(),
        minSize: 130,
        cell: (info) => {
          const value = info.getValue();
          return (
            <TableCell>
              {renderOptionalTableValue(value, m.activity_log_missing_location())}
            </TableCell>
          );
        },
      }),
      columnHelper.accessor('event', {
        header: m.activity_log_col_event(),
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
        header: m.activity_log_col_module(),
        minSize: 120,
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
        header: m.activity_log_col_description(),
        minSize: 300,
        size: 300,
        enableResizing: true,
        meta: {
          flex: true,
        },
        cell: (info) => {
          const value = info.getValue();
          return (
            <TableCell>
              <span>{value}</span>
            </TableCell>
          );
        },
      }),
      columnHelper.display({
        id: 'fill',
        minSize: 40,
        size: 40,
        enableResizing: false,
        cell: () => (
          <TableCell flex>
            <span></span>
          </TableCell>
        ),
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
        title={m.activity_log_empty_title()}
        subtitle={m.activity_log_empty_subtitle()}
      />
    );

  return (
    <>
      <TableTop text={m.activity_log_table_title()}></TableTop>
      <TableBody
        table={table}
        hasNextPage={hasNextPage}
        onNextPage={onNextPage}
        loadingNextPage={loadingNextPage}
      />
    </>
  );
};
