import './style.scss';
import {
  type ColumnFiltersState,
  createColumnHelper,
  getCoreRowModel,
  type OnChangeFn,
  type SortingState,
  useReactTable,
} from '@tanstack/react-table';
import clsx from 'clsx';
import { useMemo } from 'react';
import { m } from '../../paraglide/messages';
import {
  AclProtocolName,
  ConnectionDirection,
  type ConnectionDirectionValue,
  type ConnectionEvent,
  type ConnectionEventFilterOption,
  type ConnectionEventFilterOptions,
  ConnectionStatus,
  type ConnectionStatusValue,
} from '../../shared/api/types';
import type { SelectionOption } from '../../shared/components/SelectionSection/type';
import { TransferText } from '../../shared/components/TransferText/TransferText';
import { Badge } from '../../shared/defguard-ui/components/Badge/Badge';
import { DateInput } from '../../shared/defguard-ui/components/DateInput/DateInput';
import type { DateRange } from '../../shared/defguard-ui/components/DateInput/types';
import { EmptyStateFlexible } from '../../shared/defguard-ui/components/EmptyStateFlexible/EmptyStateFlexible';
import { Icon } from '../../shared/defguard-ui/components/Icon';
import { Search } from '../../shared/defguard-ui/components/Search/Search';
import { TableBody } from '../../shared/defguard-ui/components/table/TableBody/TableBody';
import { TableCell } from '../../shared/defguard-ui/components/table/TableCell/TableCell';
import { TableTop } from '../../shared/defguard-ui/components/table/TableTop/TableTop';
import { isPresent } from '../../shared/defguard-ui/utils/isPresent';
import { displayDate } from '../../shared/utils/displayDate';
import { formatIpForDisplay } from '../../shared/utils/formatIpForDisplay';

type RowData = ConnectionEvent;

const columnHelper = createColumnHelper<RowData>();
const timestampFormat = 'YYYY-MM-DD HH:mm:ss';

// Protocols ACL rules can match, plus ICMPv6 which Gateways may report.
const protocolNames: Record<number, string> = { ...AclProtocolName, 58: 'ICMPv6' };

const displayProtocol = (protocol: number) =>
  protocolNames[protocol] ?? protocol.toString();

const statusLabel = (status: ConnectionStatusValue) =>
  status === ConnectionStatus.Blocked
    ? m.traffic_status_blocked()
    : m.traffic_status_allowed();

const directionLabel = (direction: ConnectionDirectionValue) =>
  direction === ConnectionDirection.Inbound
    ? m.traffic_direction_inbound()
    : m.traffic_direction_outbound();

const statusFilterOptions: SelectionOption<ConnectionStatusValue>[] = Object.values(
  ConnectionStatus,
).map((status) => ({
  id: status,
  label: statusLabel(status),
}));

const directionFilterOptions: SelectionOption<ConnectionDirectionValue>[] = Object.values(
  ConnectionDirection,
).map((direction) => ({
  id: direction,
  label: directionLabel(direction),
}));

const protocolFilterOptions: SelectionOption<number>[] = Object.entries(
  protocolNames,
).map(([id, label]) => ({ id: Number(id), label }));

const toSelectionOptions = (
  options: ConnectionEventFilterOption[] | undefined,
): SelectionOption<number>[] =>
  options?.map(({ id, name, context }) => ({
    id,
    label: isPresent(context) ? `${name} (${context})` : name,
    searchFields: isPresent(context) ? [name, context] : [name],
  })) ?? [];

/** Shows `value`, or the muted `fallback` when it is missing. */
const TextCell = ({
  value,
  fallback = '—',
}: {
  value: string | null | undefined;
  fallback?: string;
}) => {
  const text = value ?? fallback;
  return (
    <TableCell>
      <span className={clsx({ 'traffic-muted': !isPresent(value) })} title={text}>
        {text}
      </span>
    </TableCell>
  );
};

const TransferCell = ({
  bytes,
  variant,
}: {
  bytes: number;
  variant: 'upload' | 'download';
}) => (
  <TableCell>
    <TransferText data={bytes} variant={variant} icon />
  </TableCell>
);

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
  filterOptions: ConnectionEventFilterOptions | undefined;
  dateRange: DateRange | null;
  onDateRangeChange: (value: DateRange | null) => void;
}

/** Column IDs of filterable columns match the API filter parameter names. */
export const TrafficLogsTable = ({
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
  filterOptions,
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
        header: m.traffic_col_timestamp(),
        enableSorting: true,
        minSize: 170,
        cell: (info) => (
          <TextCell value={displayDate(info.getValue(), timestampFormat)} />
        ),
      }),
      columnHelper.accessor('status', {
        id: 'status',
        header: m.traffic_col_status(),
        enableColumnFilter: true,
        minSize: 120,
        meta: { filterOptions: statusFilterOptions },
        cell: (info) => {
          const status = info.getValue();
          const blocked = status === ConnectionStatus.Blocked;
          return (
            <TableCell>
              <Badge
                text={statusLabel(status)}
                variant={blocked ? 'critical' : 'neutral'}
                showIcon={blocked}
                icon="lock-closed"
              />
            </TableCell>
          );
        },
      }),
      columnHelper.accessor('source_ip', {
        header: m.traffic_col_source(),
        minSize: 140,
        cell: (info) => <TextCell value={formatIpForDisplay(info.getValue())} />,
      }),
      columnHelper.accessor('device_name', {
        id: 'device',
        header: m.traffic_col_device(),
        enableColumnFilter: true,
        minSize: 140,
        meta: { filterOptions: toSelectionOptions(filterOptions?.devices) },
        cell: (info) => (
          <TextCell value={info.getValue()} fallback={m.traffic_unknown_device()} />
        ),
      }),
      columnHelper.accessor('direction', {
        id: 'direction',
        header: m.traffic_col_direction(),
        enableColumnFilter: true,
        minSize: 130,
        meta: { filterOptions: directionFilterOptions },
        cell: (info) => {
          const direction = info.getValue();
          return (
            <TableCell>
              <div className="traffic-direction">
                <Icon
                  icon="arrow-big"
                  size={16}
                  rotationDirection={
                    direction === ConnectionDirection.Inbound ? 'right' : 'left'
                  }
                />
                <span>{directionLabel(direction)}</span>
              </div>
            </TableCell>
          );
        },
      }),
      columnHelper.accessor('gateway_name', {
        id: 'gateway',
        header: m.traffic_col_gateway(),
        enableColumnFilter: true,
        minSize: 140,
        meta: { filterOptions: toSelectionOptions(filterOptions?.gateways) },
        cell: (info) => (
          <TextCell
            value={info.getValue()}
            fallback={m.traffic_deleted_gateway({ id: info.row.original.gateway_id })}
          />
        ),
      }),
      columnHelper.accessor('protocol', {
        id: 'protocol',
        header: m.traffic_col_protocol(),
        enableColumnFilter: true,
        minSize: 110,
        meta: { filterOptions: protocolFilterOptions },
        cell: (info) => <TextCell value={displayProtocol(info.getValue())} />,
      }),
      columnHelper.accessor('destination_port', {
        header: m.traffic_col_port(),
        minSize: 70,
        cell: (info) => <TextCell value={info.getValue()?.toString()} />,
      }),
      columnHelper.accessor('destination_ip', {
        header: m.traffic_col_destination(),
        minSize: 140,
        cell: (info) => <TextCell value={formatIpForDisplay(info.getValue())} />,
      }),
      columnHelper.accessor('rule_name', {
        id: 'rule',
        header: m.traffic_col_rule(),
        enableColumnFilter: true,
        minSize: 140,
        meta: { filterOptions: toSelectionOptions(filterOptions?.rules) },
        cell: (info) => {
          const ruleId = info.row.original.rule_id;
          return (
            <TextCell
              value={info.getValue()}
              fallback={
                isPresent(ruleId)
                  ? m.traffic_deleted_rule({ id: ruleId })
                  : m.traffic_no_rule()
              }
            />
          );
        },
      }),
      columnHelper.accessor('location_name', {
        id: 'location',
        header: m.traffic_col_location(),
        enableColumnFilter: true,
        minSize: 140,
        meta: { filterOptions: toSelectionOptions(filterOptions?.locations) },
        cell: (info) => <TextCell value={info.getValue()} />,
      }),
      columnHelper.accessor('download_bytes', {
        header: m.traffic_col_incoming(),
        enableSorting: true,
        minSize: 120,
        cell: (info) => <TransferCell bytes={info.getValue()} variant="download" />,
      }),
      columnHelper.accessor('upload_bytes', {
        header: m.traffic_col_outgoing(),
        enableSorting: true,
        minSize: 120,
        cell: (info) => <TransferCell bytes={info.getValue()} variant="upload" />,
      }),
      columnHelper.accessor('count', {
        header: m.traffic_col_count(),
        enableSorting: true,
        minSize: 90,
        cell: (info) => <TextCell value={info.getValue().toLocaleString()} />,
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
    [filterOptions],
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
    // Only columns that explicitly opt in can be sorted or filtered.
    enableSorting: true,
    manualSorting: true,
    enableSortingRemoval: false,
    onSortingChange,
    manualFiltering: true,
    onColumnFiltersChange,
    defaultColumn: {
      enableSorting: false,
      enableColumnFilter: false,
    },
    meta: {
      filterMessages: tableFilterMessages,
    },
  });

  return (
    <>
      <TableTop text={m.traffic_table_title()}>
        <Search
          placeholder={m.controls_search()}
          initialValue={search}
          onChange={onSearchChange}
        />
        <DateInput
          placeholder={m.traffic_date_range_placeholder()}
          testId="traffic-date-input"
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
          title={m.traffic_empty_title()}
          subtitle={m.traffic_empty_subtitle()}
        />
      )}
    </>
  );
};
