import { keepPreviousData, useInfiniteQuery } from '@tanstack/react-query';
import type { SortingState } from '@tanstack/react-table';
import { useMemo, useState } from 'react';
import { m } from '../../paraglide/messages';
import api from '../../shared/api/api';
import { Page } from '../../shared/components/Page/Page';
import { SizedBox } from '../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../shared/defguard-ui/types';
import { isPresent } from '../../shared/defguard-ui/utils/isPresent';
import { TablePageLayout } from '../../shared/layout/TablePageLayout/TablePageLayout';
import { ActivityLogTable } from './ActivityLogTable';

export const ActivityLogPage = () => {
  const [search, setSearch] = useState('');
  const [sortingState, setSortingState] = useState<SortingState>([
    { id: 'timestamp', desc: true },
  ]);

  const activeSorting = sortingState[0];

  const { data, fetchNextPage, isFetchingNextPage } = useInfiniteQuery({
    queryKey: ['activity-log', { search, sortingState }],
    initialPageParam: 1,
    queryFn: ({ pageParam }) =>
      api.getActivityLog({
        page: pageParam,
        search: search.length > 0 ? search : undefined,
        // sort_by is typed as keyof string due to RequestSortParams<ActivityLogSortKey> quirk
        sort_by: activeSorting?.id as never,
        sort_order: activeSorting ? (activeSorting.desc ? 'desc' : 'asc') : undefined,
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
          />
        )}
      </TablePageLayout>
    </Page>
  );
};
