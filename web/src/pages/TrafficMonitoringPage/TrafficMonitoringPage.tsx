import { keepPreviousData, useInfiniteQuery, useQuery } from '@tanstack/react-query';
import type { ColumnFiltersState, SortingState } from '@tanstack/react-table';
import { useMemo, useState } from 'react';
import { m } from '../../paraglide/messages';
import api from '../../shared/api/api';
import { type ConnectionEventSortKey, SortDirection } from '../../shared/api/types';
import { Page } from '../../shared/components/Page/Page';
import { TableSkeleton } from '../../shared/components/skeleton/TableSkeleton/TableSkeleton';
import type { DateRange } from '../../shared/defguard-ui/components/DateInput/types';
import { EmptyStateFlexible } from '../../shared/defguard-ui/components/EmptyStateFlexible/EmptyStateFlexible';
import { SizedBox } from '../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../shared/defguard-ui/types';
import { isPresent } from '../../shared/defguard-ui/utils/isPresent';
import { TablePageLayout } from '../../shared/layout/TablePageLayout/TablePageLayout';
import {
  getConnectionEventFilterOptionsQueryOptions,
  getLicenseInfoQueryOptions,
} from '../../shared/query';
import { mapColumnFiltersToApiParams } from '../../shared/utils/columnFilters';
import { canUseEnterpriseFeature, licenseActionCheck } from '../../shared/utils/license';
import { TrafficLogsTable } from './TrafficLogsTable';

const TrafficLogs = () => {
  const [search, setSearch] = useState('');
  const [dateRange, setDateRange] = useState<DateRange | null>(null);
  const [sortingState, setSortingState] = useState<SortingState>([
    { id: 'timestamp', desc: true },
  ]);
  const [columnFilters, setColumnFilters] = useState<ColumnFiltersState>([]);

  const { data: filterOptions } = useQuery(getConnectionEventFilterOptionsQueryOptions);

  const filterParams = useMemo(
    () => mapColumnFiltersToApiParams(columnFilters),
    [columnFilters],
  );

  const activeSorting = sortingState[0];

  const { data, fetchNextPage, hasNextPage, isFetchingNextPage } = useInfiniteQuery({
    queryKey: ['connection-event', { search, dateRange, sortingState, columnFilters }],
    initialPageParam: 1,
    queryFn: ({ pageParam }) =>
      api.getConnectionEvents({
        page: pageParam,
        search: search || undefined,
        from: dateRange?.start.toISOString(),
        until: dateRange?.end.toISOString(),
        sort_by: activeSorting?.id as ConnectionEventSortKey,
        sort_order: activeSorting
          ? activeSorting.desc
            ? SortDirection.DESC
            : SortDirection.ASC
          : undefined,
        ...filterParams,
      }),
    placeholderData: keepPreviousData,
    getNextPageParam: (lastPage) => lastPage?.pagination.next_page,
    // Poll like Gateway status, so new traffic shows up without reloading.
    refetchInterval: 30_000,
    refetchOnReconnect: true,
  });

  const flatData = useMemo(
    () => data?.pages.flatMap((page) => page.data) ?? null,
    [data],
  );

  return (
    <TablePageLayout>
      {isPresent(flatData) && (
        <TrafficLogsTable
          data={flatData}
          loadingNextPage={isFetchingNextPage}
          onNextPage={() => {
            fetchNextPage();
          }}
          hasNextPage={hasNextPage}
          search={search}
          onSearchChange={setSearch}
          sortingState={sortingState}
          onSortingChange={setSortingState}
          columnFilters={columnFilters}
          onColumnFiltersChange={setColumnFilters}
          filterOptions={filterOptions}
          dateRange={dateRange}
          onDateRangeChange={setDateRange}
        />
      )}
    </TablePageLayout>
  );
};

export const TrafficMonitoringPage = () => {
  const { data: licenseInfo } = useQuery(getLicenseInfoQueryOptions);
  const licenseCheck = useMemo(
    () => (licenseInfo === undefined ? null : canUseEnterpriseFeature(licenseInfo)),
    [licenseInfo],
  );

  return (
    <Page id="traffic-monitoring-page" title={m.traffic_page_title()}>
      <SizedBox height={ThemeSpacing.Xl3} />
      {licenseCheck === null && <TableSkeleton />}
      {licenseCheck?.result === true && <TrafficLogs />}
      {licenseCheck?.result === false && (
        <EmptyStateFlexible
          icon="log"
          title={m.traffic_unavailable_title()}
          subtitle={m.traffic_unavailable_subtitle()}
          primaryAction={{
            text: m.traffic_unavailable_action(),
            onClick: () => {
              licenseActionCheck(licenseCheck, () => {});
            },
          }}
        />
      )}
    </Page>
  );
};
