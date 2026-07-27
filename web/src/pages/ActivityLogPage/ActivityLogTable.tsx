import {
  type ColumnFiltersState,
  createColumnHelper,
  getCoreRowModel,
  type OnChangeFn,
  type SortingState,
  useReactTable,
} from '@tanstack/react-table';
import { useMemo } from 'react';
import { m } from '../../paraglide/messages';
import {
  ActivityLogEventType,
  type ActivityLogEventTypeValue,
  ActivityLogModule,
  type ActivityLogModuleValue,
  activityLogEventDisplay,
} from '../../shared/api/activity-log-types';
import type { ActivityLogEvent } from '../../shared/api/types';
import type { SelectionOption } from '../../shared/components/SelectionSection/type';
import { DateInput } from '../../shared/defguard-ui/components/DateInput/DateInput';
import type { DateRange } from '../../shared/defguard-ui/components/DateInput/types';
import { EmptyStateFlexible } from '../../shared/defguard-ui/components/EmptyStateFlexible/EmptyStateFlexible';
import { Search } from '../../shared/defguard-ui/components/Search/Search';
import { TableBody } from '../../shared/defguard-ui/components/table/TableBody/TableBody';
import { TableCell } from '../../shared/defguard-ui/components/table/TableCell/TableCell';
import { TableTop } from '../../shared/defguard-ui/components/table/TableTop/TableTop';
import { isPresent } from '../../shared/defguard-ui/utils/isPresent';
import { displayDate } from '../../shared/utils/displayDate';
import { formatIpForDisplay } from '../../shared/utils/formatIpForDisplay';

type RowData = ActivityLogEvent;

const columnHelper = createColumnHelper<RowData>();
const missingValuePlaceholder = '—';
const activityLogTimestampFormat = 'DD/MM/YYYY | HH:mm:ss';

const eventFilterOptions: SelectionOption<ActivityLogEventTypeValue>[] = Object.values(
  ActivityLogEventType,
).map((event) => ({
  id: event,
  label: activityLogEventDisplay[event],
  searchFields: [activityLogEventDisplay[event]],
}));

const moduleFilterOptions: SelectionOption<ActivityLogModuleValue>[] = Object.values(
  ActivityLogModule,
).map((module) => ({
  id: module,
  label: module,
  searchFields: [module],
}));

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
  hasNextPage: boolean;
  loadingNextPage: boolean;
  onNextPage: () => void;
  search: string;
  onSearchChange: (val: string) => void;
  sortingState: SortingState;
  onSortingChange: OnChangeFn<SortingState>;
  columnFilters: ColumnFiltersState;
  onColumnFiltersChange: OnChangeFn<ColumnFiltersState>;
  locationFilterOptions: SelectionOption<string>[];
  dateRange: DateRange | null;
  onDateRangeChange: (value: DateRange | null) => void;
}

export const ActivityLogTable = ({
  data,
  loadingNextPage,
  hasNextPage,
  onNextPage,
  search,
  onSearchChange,
  sortingState,
  onSortingChange,
  columnFilters,
  onColumnFiltersChange,
  locationFilterOptions,
  dateRange,
  onDateRangeChange,
}: Props) => {
  const tableFilterMessages = useMemo(
    () => ({
      searchPlaceholder: m.controls_search(),
      clearButton: m.controls_reset(),
      applyButton: m.controls_submit(),
      emptyState: m.search_empty_common_title(),
    }),
    [],
  );
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
        enableSorting: true,
        minSize: 150,
        cell: (info) => (
          <TableCell>
            <span>{info.getValue()}</span>
          </TableCell>
        ),
      }),
      columnHelper.accessor('ip', {
        header: m.activity_log_col_ip(),
        enableSorting: true,
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
        enableSorting: true,
        enableColumnFilter: true,
        minSize: 130,
        meta: {
          filterOptions: locationFilterOptions,
        },
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
        enableSorting: true,
        enableColumnFilter: true,
        minSize: 190,
        meta: {
          filterOptions: eventFilterOptions,
        },
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
        enableSorting: true,
        enableColumnFilter: true,
        minSize: 120,
        meta: {
          filterOptions: moduleFilterOptions,
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
    [locationFilterOptions],
  );

  const table = useReactTable({
    state: {
      sorting: sortingState,
      columnFilters,
    },
    data,
    columns,
    columnResizeMode: 'onChange',
    getCoreRowModel: getCoreRowModel(),
    enableRowSelection: false,
    enableExpanding: false,
    enableSorting: true,
    manualSorting: true,
    enableSortingRemoval: false,
    onSortingChange,
    manualFiltering: true,
    onColumnFiltersChange,
    meta: {
      filterMessages: tableFilterMessages,
    },
  });

  return (
    <>
      <TableTop text={m.activity_log_table_title()}>
        <Search
          placeholder={m.controls_search()}
          initialValue={search}
          onChange={onSearchChange}
        />
        <DateInput
          placeholder={m.activity_log_date_range_placeholder()}
          testId='date-input'
          labels={{
            start: m.activity_log_date_range_start(),
            end: m.activity_log_date_range_end(),
            reset: m.controls_reset(),
            cancel: m.controls_cancel(),
            apply: m.controls_apply(),
          }}
          value={dateRange}
          onChange={onDateRangeChange}
        />
      </TableTop>
      <TableBody
        table={table}
        hasNextPage={hasNextPage}
        onNextPage={onNextPage}
        loadingNextPage={loadingNextPage}
      />
      {data.length === 0 && (
        <EmptyStateFlexible
          icon="log"
          title={m.activity_log_empty_title()}
          subtitle={m.activity_log_empty_subtitle()}
        />
      )}
    </>
  );
};
