import './style.scss';

import { useInfiniteQuery } from '@tanstack/react-query';
import { useEffect, useMemo } from 'react';

import { useI18nContext } from '../../i18n/i18n-react';
import { PageContainer } from '../../shared/components/Layout/PageContainer/PageContainer';
import { PageLimiter } from '../../shared/components/Layout/PageLimiter/PageLimiter';
import { Card } from '../../shared/defguard-ui/components/Layout/Card/Card';
import { ListItemCount } from '../../shared/defguard-ui/components/Layout/ListItemCount/ListItemCount';
import { isPresent } from '../../shared/defguard-ui/utils/isPresent';
import useApi from '../../shared/hooks/useApi';
import { ActivityList } from './components/ActivityList';

export const ActivityPage = () => {
  return (
    <PageContainer id="activity-page">
      <PageLimiter>
        <PageContent />
      </PageLimiter>
    </PageContainer>
  );
};

const PageContent = () => {
  const { LL } = useI18nContext();
  const {
    auditLog: { getAuditLog },
  } = useApi();

  const {
    data,
    hasNextPage,
    isFetchingNextPage,
    fetchNextPage,
    // hasPreviousPage,
    // fetchPreviousPage,
  } = useInfiniteQuery({
    queryKey: ['audit_log'],
    initialPageParam: 1,
    queryFn: ({ pageParam }) =>
      getAuditLog({
        page: pageParam,
      }),
    getNextPageParam: (lastPage) => lastPage?.pagination?.next_page,
    getPreviousPageParam: (page) => {
      if (page.pagination.current_page !== 1) {
        return page.pagination.current_page - 1;
      }
      return undefined;
    },
  });

  const activityData = useMemo(() => {
    if (data) {
      return data.pages.map((page) => page.data).flat(1);
    }
    return undefined;
  }, [data]);

  useEffect(() => {
    console.log(data);
  }, [data]);

  return (
    <>
      <header className="page-header">
        <h1>Activity</h1>
        {/* <Search
          placeholder={LL.common.search()}
          onDebounce={(val) => {
            setSearch(val);
          }}
        /> */}
      </header>
      <div id="activity-list">
        <div className="top">
          <h2>All activity</h2>
          <ListItemCount shorten count={data?.pages[0].pagination.total_items ?? 0} />
          <div className="controls"></div>
        </div>
        <Card id="activity-list-card">
          {isPresent(activityData) && (
            <ActivityList
              data={activityData}
              hasNextPage={hasNextPage}
              isFetchingNextPage={isFetchingNextPage}
              onNextPage={() => {
                void fetchNextPage();
              }}
            />
          )}
        </Card>
      </div>
    </>
  );
};
