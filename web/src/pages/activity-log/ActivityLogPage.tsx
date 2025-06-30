import './style.scss';

import { QueryKey, useInfiniteQuery, useQuery } from '@tanstack/react-query';
import dayjs from 'dayjs';
import { range } from 'lodash-es';
import { useMemo, useState } from 'react';
import Skeleton from 'react-loading-skeleton';

import { useI18nContext } from '../../i18n/i18n-react';
import { FilterButton } from '../../shared/components/Layout/buttons/FilterButton/FilterButton';
import { PageContainer } from '../../shared/components/Layout/PageContainer/PageContainer';
import { PageLimiter } from '../../shared/components/Layout/PageLimiter/PageLimiter';
import { FilterGroupsModal } from '../../shared/components/modals/FilterGroupsModal/FilterGroupsModal';
import { FilterGroupsModalFilter } from '../../shared/components/modals/FilterGroupsModal/types';
import { Button } from '../../shared/defguard-ui/components/Layout/Button/Button';
import { ButtonSize } from '../../shared/defguard-ui/components/Layout/Button/types';
import { Card } from '../../shared/defguard-ui/components/Layout/Card/Card';
import { ListItemCount } from '../../shared/defguard-ui/components/Layout/ListItemCount/ListItemCount';
import { NoData } from '../../shared/defguard-ui/components/Layout/NoData/NoData';
import { Search } from '../../shared/defguard-ui/components/Layout/Search/Search';
import { ListSortDirection } from '../../shared/defguard-ui/components/Layout/VirtualizedList/types';
import { isPresent } from '../../shared/defguard-ui/utils/isPresent';
import { useAuthStore } from '../../shared/hooks/store/useAuthStore';
import useApi from '../../shared/hooks/useApi';
import { ActivityLogSortKey } from '../../shared/types';
import { ActivityList } from './components/ActivityList';
import { ActivityTimeRangeModal } from './components/ActivityTimeRangeModal';
import {
  ActivityLogEventType,
  activityLogEventTypeValues,
  ActivityLogModule,
  activityLogModuleValues,
} from './types';

export const ActivityLogPage = () => {
  return (
    <PageContainer id="activity-log-page">
      <PageLimiter>
        <PageContent />
      </PageLimiter>
    </PageContainer>
  );
};

const applyFilterArray = <T,>(val: Array<T> | undefined | T): undefined | Array<T> => {
  if (val && Array.isArray(val) && val.length > 0) {
    return val;
  }
};

const applyFilter = <T,>(val: T | undefined | null): T | undefined => {
  if (isPresent(val)) {
    return val;
  }
};

const applySearch = (val: string): string | undefined => {
  if (val.length > 0) return val;
  return undefined;
};

type Filters = 'event' | 'username' | 'module';

const PageContent = () => {
  const [activeFilters, setActiveFilters] = useState<
    Record<Filters, Array<number | string>>
  >({
    event: [],
    module: [],
    username: [],
  });
  const [searchValue, setSearchValue] = useState<string>('');
  const [filtersModalOpen, setFiltersModalOpen] = useState(false);
  const [from, setForm] = useState<string | null>(dayjs.utc().startOf('M').toISOString());
  const [until, setUntil] = useState<string | null>(null);
  const [timeSelectionModalOpen, setTimeSelectionModal] = useState(false);
  const [sortKey, setSortKey] = useState<ActivityLogSortKey>('timestamp');
  const [sortDirection, setSortDirection] = useState<ListSortDirection>(
    ListSortDirection.DESC,
  );
  const isAdmin = useAuthStore((s) => s.user?.is_admin ?? false);

  const activeFiltersCount = useMemo(
    () => Object.values(activeFilters).flat().length,
    [activeFilters],
  );

  const { LL } = useI18nContext();
  const localLL = LL.activity;

  const {
    activityLog: { getActivityLog },
    user: { getUsers },
  } = useApi();

  const { data: users } = useQuery({
    queryFn: getUsers,
    queryKey: ['user'],
    enabled: isAdmin,
  });

  const queryKey = useMemo(
    (): QueryKey => [
      'activity_log',
      {
        sortDirection,
        sortKey,
        from,
        until,
        searchValue,
        filters: activeFilters,
      },
    ],
    [activeFilters, from, searchValue, sortDirection, sortKey, until],
  );

  const {
    data,
    hasNextPage,
    isFetchingNextPage,
    fetchNextPage,
    isLoading,
    // hasPreviousPage,
    // fetchPreviousPage,
  } = useInfiniteQuery({
    queryKey,
    initialPageParam: 1,
    queryFn: ({ pageParam }) =>
      getActivityLog({
        page: pageParam,
        event: applyFilterArray(activeFilters.event as ActivityLogEventType[]),
        module: applyFilterArray(activeFilters.module as ActivityLogModule[]),
        username: applyFilterArray(activeFilters.username as string[]),
        sort_order: sortDirection,
        sort_by: sortKey,
        search: applySearch(searchValue),
        from: applyFilter(from),
        until: applyFilter(until),
      }),
    getNextPageParam: (lastPage) => lastPage?.pagination?.next_page,
    getPreviousPageParam: (page) => {
      if (page.pagination.current_page !== 1) {
        return page.pagination.current_page - 1;
      }
      return undefined;
    },
  });

  const filterOptions = useMemo(() => {
    const res: Record<string, FilterGroupsModalFilter> = {};
    if (users) {
      res['users'] = {
        label: 'Users',
        identifier: 'username',
        order: 3,
        items: users.map((user) => ({
          label: `${user.first_name} ${user.last_name} (${user.username})`,
          searchValues: [user.first_name, user.username, user.last_name, user.email],
          value: user.username,
        })),
      };
    }
    res['module'] = {
      identifier: 'module',
      label: 'Module',
      order: 2,
      items: activityLogModuleValues.map((activityLogModule) => {
        const translation = LL.enums.activityLogModule[activityLogModule]();
        return {
          label: translation,
          searchValues: [translation],
          value: activityLogModule,
        };
      }),
    };
    res['event'] = {
      identifier: 'event',
      label: 'Event',
      order: 1,
      items: activityLogEventTypeValues.map((eventType) => {
        const translation = LL.enums.activityLogEventType[eventType]();
        return {
          label: translation,
          searchValues: [translation],
          value: eventType,
        };
      }),
    };
    return res;
  }, [LL.enums, users]);

  const activityData = useMemo(() => {
    if (data) {
      return data.pages.map((page) => page.data).flat(1);
    }
    return undefined;
  }, [data]);

  return (
    <>
      <header className="page-header">
        <h1>{localLL.title()}</h1>
      </header>
      <div id="activity-list">
        <div className="top">
          <h2>{localLL.list.allLabel()}</h2>
          <ListItemCount shorten count={data?.pages[0].pagination.total_items ?? 0} />
          <div className="controls">
            <Search
              placeholder={LL.common.search()}
              initialValue={searchValue}
              onDebounce={(search) => {
                setSearchValue(search);
              }}
            />
            <FilterButton
              activeFiltersCount={activeFiltersCount}
              onClick={() => {
                setFiltersModalOpen(true);
              }}
            />
            <Button
              size={ButtonSize.SMALL}
              text={LL.common.controls.timeRange()}
              icon={
                <svg
                  xmlns="http://www.w3.org/2000/svg"
                  width="18"
                  height="18"
                  viewBox="0 0 22 22"
                  fill="none"
                >
                  <path
                    d="M14.207 12.59L11.697 10.708V6.87399C11.6809 6.7003 11.6005 6.53889 11.4715 6.42138C11.3426 6.30387 11.1744 6.23873 11 6.23873C10.8256 6.23873 10.6574 6.30387 10.5285 6.42138C10.3995 6.53889 10.3191 6.7003 10.303 6.87399V11.057C10.3032 11.1653 10.3285 11.272 10.3769 11.3688C10.4253 11.4656 10.4955 11.5499 10.582 11.615L13.371 13.706C13.5196 13.7977 13.6971 13.8305 13.8687 13.7981C14.0403 13.7656 14.1936 13.6702 14.2984 13.5305C14.4033 13.3909 14.4521 13.2171 14.4354 13.0432C14.4186 12.8694 14.3376 12.7081 14.208 12.591L14.207 12.59Z"
                    fill="#899CA8"
                  />
                  <path
                    d="M11 1.99999C9.21997 1.99999 7.47991 2.52783 5.99987 3.51677C4.51983 4.5057 3.36628 5.91131 2.68509 7.55584C2.0039 9.20038 1.82567 11.01 2.17294 12.7558C2.5202 14.5016 3.37737 16.1053 4.63604 17.364C5.89472 18.6226 7.49836 19.4798 9.24419 19.8271C10.99 20.1743 12.7996 19.9961 14.4442 19.3149C16.0887 18.6337 17.4943 17.4802 18.4832 16.0001C19.4722 14.5201 20 12.78 20 11C19.9974 8.61386 19.0483 6.32621 17.361 4.63896C15.6738 2.9517 13.3861 2.00264 11 1.99999ZM11 18.606C9.49568 18.606 8.02514 18.1599 6.77434 17.3242C5.52354 16.4884 4.54866 15.3005 3.97298 13.9107C3.3973 12.5209 3.24667 10.9916 3.54015 9.51614C3.83363 8.04072 4.55803 6.68546 5.62175 5.62174C6.68547 4.55802 8.04073 3.83362 9.51615 3.54014C10.9916 3.24666 12.5209 3.39729 13.9107 3.97297C15.3005 4.54865 16.4884 5.52353 17.3242 6.77433C18.1599 8.02513 18.606 9.49567 18.606 11C18.6036 13.0165 17.8015 14.9497 16.3756 16.3756C14.9497 17.8015 13.0165 18.6036 11 18.606Z"
                    fill="#899CA8"
                  />
                </svg>
              }
              onClick={() => {
                setTimeSelectionModal(true);
              }}
            />
          </div>
        </div>
        <Card id="activity-list-card">
          {isPresent(activityData) && activityData.length > 0 && (
            <ActivityList
              sortDirection={sortDirection}
              sortKey={sortKey}
              onSortChange={(sortKey, sortDirection) => {
                setSortDirection(sortDirection);
                setSortKey(sortKey as ActivityLogSortKey);
              }}
              data={activityData}
              hasNextPage={hasNextPage}
              isFetchingNextPage={isFetchingNextPage}
              onNextPage={() => {
                void fetchNextPage();
              }}
            />
          )}
          {!isPresent(activityData) && isLoading && (
            <div className="activity-list-skeleton">
              {range(10).map((index) => (
                <Skeleton key={index} />
              ))}
            </div>
          )}
          {(activeFiltersCount > 0 || searchValue.length > 0) &&
            isPresent(activityData) &&
            activityData.length === 0 && <NoData customMessage="" />}
        </Card>
      </div>
      <FilterGroupsModal
        data={filterOptions}
        isOpen={filtersModalOpen}
        currentState={activeFilters}
        onCancel={() => {
          setFiltersModalOpen(false);
        }}
        onSubmit={(state) => {
          setActiveFilters(state as Record<Filters, number[]>);
          setFiltersModalOpen(false);
        }}
      />
      <ActivityTimeRangeModal
        activityFrom={from}
        activityUntil={until}
        isOpen={timeSelectionModalOpen}
        onOpenChange={setTimeSelectionModal}
        onChange={(from, until) => {
          setForm(from);
          setUntil(until);
        }}
      />
    </>
  );
};
