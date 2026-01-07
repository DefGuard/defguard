import { useInfiniteQuery } from '@tanstack/react-query';
import { useMemo } from 'react';
import api from '../../shared/api/api';
import { Page } from '../../shared/components/Page/Page';
import { SizedBox } from '../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../shared/defguard-ui/types';
import { isPresent } from '../../shared/defguard-ui/utils/isPresent';
import { ActivityLogTable } from './ActivityLogTable';

export const ActivityLogPage = () => {
  const { data, fetchNextPage, isFetchingNextPage } = useInfiniteQuery({
    queryKey: ['activity-log'],
    initialPageParam: 1,
    queryFn: ({ pageParam }) =>
      api.getActivityLog({
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
    () => flatQueryData?.flatMap((page) => page.data) ?? null,
    [flatQueryData],
  );

  const lastItem = flatQueryData ? flatQueryData[flatQueryData?.length - 1] : null;
  const pagination = lastItem ? lastItem.pagination : null;

  return (
    <Page id="activity-log-page" title={`Activity log`}>
      <SizedBox height={ThemeSpacing.Xl3} />
      {isPresent(flatData) && isPresent(pagination) && (
        <ActivityLogTable
          data={flatData}
          pagination={pagination}
          filters={{}}
          loadingNextPage={isFetchingNextPage}
          onNextPage={() => {
            fetchNextPage();
          }}
          hasNextPage={pagination.next_page !== null}
        />
      )}
    </Page>
  );
};
