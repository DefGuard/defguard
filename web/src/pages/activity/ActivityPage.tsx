import './style.scss';

import { useInfiniteQuery, useQuery } from '@tanstack/react-query';
import dayjs from 'dayjs';
import { range } from 'lodash-es';
import { useMemo, useState } from 'react';
import Skeleton from 'react-loading-skeleton';

import { useI18nContext } from '../../i18n/i18n-react';
import { PageContainer } from '../../shared/components/Layout/PageContainer/PageContainer';
import { PageLimiter } from '../../shared/components/Layout/PageLimiter/PageLimiter';
import { FilterGroupsModal } from '../../shared/components/modals/FilterGroupsModal/FilterGroupsModal';
import { FilterGroupsModalFilter } from '../../shared/components/modals/FilterGroupsModal/types';
import { Button } from '../../shared/defguard-ui/components/Layout/Button/Button';
import { ButtonSize } from '../../shared/defguard-ui/components/Layout/Button/types';
import { Card } from '../../shared/defguard-ui/components/Layout/Card/Card';
import { ListItemCount } from '../../shared/defguard-ui/components/Layout/ListItemCount/ListItemCount';
import { Search } from '../../shared/defguard-ui/components/Layout/Search/Search';
import { ListSortDirection } from '../../shared/defguard-ui/components/Layout/VirtualizedList/types';
import { isPresent } from '../../shared/defguard-ui/utils/isPresent';
import useApi from '../../shared/hooks/useApi';
import { AuditLogSortKey } from '../../shared/types';
import { ActivityList } from './components/ActivityList';
import { ActivityTimeRangeModal } from './components/ActivityTimeRangeModal';
import {
  AuditEventType,
  auditEventTypeValues,
  AuditModule,
  auditModuleValues,
} from './types';

export const ActivityPage = () => {
  return (
    <PageContainer id="activity-page">
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
  const [sortKey, setSortKey] = useState<AuditLogSortKey>('timestamp');
  const [sortDirection, setSortDirection] = useState<ListSortDirection>(
    ListSortDirection.DESC,
  );

  const { LL } = useI18nContext();

  const {
    auditLog: { getAuditLog },
    user: { getUsers },
  } = useApi();

  const { data: users } = useQuery({
    queryFn: getUsers,
    queryKey: ['user'],
  });

  const {
    data,
    hasNextPage,
    isFetchingNextPage,
    fetchNextPage,
    isLoading,
    // hasPreviousPage,
    // fetchPreviousPage,
  } = useInfiniteQuery({
    queryKey: [
      'audit_log',
      sortDirection,
      sortKey,
      activeFilters.event,
      activeFilters.module,
      activeFilters.username,
      from,
      until,
      searchValue,
    ],
    initialPageParam: 1,
    queryFn: ({ pageParam }) =>
      getAuditLog({
        page: pageParam,
        event: applyFilterArray(activeFilters.event as AuditEventType[]),
        module: applyFilterArray(activeFilters.module as AuditModule[]),
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
      items: auditModuleValues.map((auditModule) => {
        const translation = LL.enums.auditModule[auditModule]();
        return {
          label: translation,
          searchValues: [translation],
          value: auditModule,
        };
      }),
    };
    res['event'] = {
      identifier: 'event',
      label: 'Event',
      order: 1,
      items: auditEventTypeValues.map((eventType) => {
        const translation = LL.enums.auditEventType[eventType]();
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
          <div className="controls">
            <Search
              placeholder="Search"
              initialValue={searchValue}
              onDebounce={(search) => {
                setSearchValue(search);
              }}
            />
            <Button
              size={ButtonSize.SMALL}
              text="Filters"
              onClick={() => {
                setFiltersModalOpen(true);
              }}
            />
            <Button
              size={ButtonSize.SMALL}
              text="Time"
              onClick={() => {
                setTimeSelectionModal(true);
              }}
            />
          </div>
        </div>
        <Card id="activity-list-card">
          {isPresent(activityData) && (
            <ActivityList
              sortDirection={sortDirection}
              sortKey={sortKey}
              onSortChange={(sortKey, sortDirection) => {
                setSortDirection(sortDirection);
                setSortKey(sortKey as AuditLogSortKey);
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
