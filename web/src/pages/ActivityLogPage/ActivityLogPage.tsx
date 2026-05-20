import { keepPreviousData, useInfiniteQuery, useQuery } from '@tanstack/react-query';
import type { ColumnFiltersState, SortingState } from '@tanstack/react-table';
import { useMemo, useState } from 'react';
import { m } from '../../paraglide/messages';
import type {
  ActivityLogEventTypeValue,
  ActivityLogModuleValue,
} from '../../shared/api/activity-log-types';
import api from '../../shared/api/api';
import { Page } from '../../shared/components/Page/Page';
import { SizedBox } from '../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../shared/defguard-ui/types';
import { isPresent } from '../../shared/defguard-ui/utils/isPresent';
import { TablePageLayout } from '../../shared/layout/TablePageLayout/TablePageLayout';
import { getLocationsQueryOptions } from '../../shared/query';
import { ActivityLogTable } from './ActivityLogTable';

export const ActivityLogPage = () => {
  const [search, setSearch] = useState('');
  const [sortingState, setSortingState] = useState<SortingState>([
    { id: 'timestamp', desc: true },
  ]);
  const [columnFilters, setColumnFilters] = useState<ColumnFiltersState>([]);

  const { data: locations } = useQuery(getLocationsQueryOptions);
  const locationFilterOptions = useMemo(
    () =>
      locations?.map((loc) => ({
        id: loc.name,
        label: loc.name,
        searchFields: [loc.name],
      })) ?? [],
    [locations],
  );

  const activeSorting = sortingState[0];

  const eventFilter = columnFilters.find((f) => f.id === 'event')?.value as
    | ActivityLogEventTypeValue[]
    | undefined;
  const moduleFilter = columnFilters.find((f) => f.id === 'module')?.value as
    | ActivityLogModuleValue[]
    | undefined;
  const locationFilter = columnFilters.find((f) => f.id === 'location')?.value as
    | string[]
    | undefined;
  const { data, fetchNextPage, isFetchingNextPage } = useInfiniteQuery({
    queryKey: ['activity-log', { search, sortingState, columnFilters }],
    initialPageParam: 1,
    queryFn: ({ pageParam }) =>
      api.getActivityLog({
        page: pageParam,
        search: search.length > 0 ? search : undefined,
        // sort_by is typed as keyof string due to RequestSortParams<ActivityLogSortKey> quirk
        sort_by: activeSorting?.id as never,
        sort_order: activeSorting ? (activeSorting.desc ? 'desc' : 'asc') : undefined,
        event: eventFilter,
        module: moduleFilter,
        location: locationFilter,
      }),
    placeholderData: keepPreviousData,
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
    () => flatQueryData?.flatMap((page) => page.data) ?? null,
    [flatQueryData],
  );

  const lastItem = flatQueryData ? flatQueryData[flatQueryData?.length - 1] : null;
  const pagination = lastItem ? lastItem.pagination : null;

  return (
    <Page id="activity-log-page" title={m.cmp_nav_item_activity_log()}>
      <SizedBox height={ThemeSpacing.Xl3} />
      <TablePageLayout>
        {isPresent(flatData) && isPresent(pagination) && (
          <ActivityLogTable
            data={flatData}
            loadingNextPage={isFetchingNextPage}
            onNextPage={() => {
              fetchNextPage();
            }}
            hasNextPage={pagination.next_page !== null}
            search={search}
            onSearchChange={setSearch}
            sortingState={sortingState}
            onSortingChange={setSortingState}
            columnFilters={columnFilters}
            onColumnFiltersChange={setColumnFilters}
            locationFilterOptions={locationFilterOptions}
          />
        )}
      </TablePageLayout>
    </Page>
  );
};
