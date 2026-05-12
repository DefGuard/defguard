import { useInfiniteQuery, useSuspenseQuery } from '@tanstack/react-query';
import type { ColumnFiltersState } from '@tanstack/react-table';
import { Suspense, useMemo, useState } from 'react';
import { m } from '../../paraglide/messages';
import api from '../../shared/api/api';
import { Page } from '../../shared/components/Page/Page';
import { TableSkeleton } from '../../shared/components/skeleton/TableSkeleton/TableSkeleton';
import type { ButtonProps } from '../../shared/defguard-ui/components/Button/types';
import { EmptyStateFlexible } from '../../shared/defguard-ui/components/EmptyStateFlexible/EmptyStateFlexible';
import { SizedBox } from '../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../shared/defguard-ui/types';
import { TablePageLayout } from '../../shared/layout/TablePageLayout/TablePageLayout';
import { getLicenseInfoQueryOptions } from '../../shared/query';
import { canUseEnterpriseFeature, licenseActionCheck } from '../../shared/utils/license';
import { PostureChecksTable } from './PostureChecksTable';
import {
  getPostureCheckTableFilterMessages,
  isPostureCheckFilterValue,
  mapApiDevicePostureToRow,
  mapPostureCheckFilterValueToRequestValue,
} from './postureChecks';
import type { PostureCheckFilterValue } from './types';

const mapColumnFiltersToRequest = (columnFilters: ColumnFiltersState) => {
  const result: Record<string, string[]> = {};

  for (const filter of columnFilters) {
    if (Array.isArray(filter.value) && filter.value.length > 0) {
      result[filter.id] = filter.value
        .filter(
          (value): value is PostureCheckFilterValue =>
            typeof value === 'string' && isPostureCheckFilterValue(value),
        )
        .map(mapPostureCheckFilterValueToRequestValue);
    }
  }

  return result;
};

const PostureChecksContent = () => {
  const { data: licenseInfo, isFetching: licenseInfoFetching } = useSuspenseQuery(
    getLicenseInfoQueryOptions,
  );
  const [columnFilters, setColumnFilters] = useState<ColumnFiltersState>([]);
  const requestFilters = useMemo(
    () => mapColumnFiltersToRequest(columnFilters),
    [columnFilters],
  );
  const filterMessages = useMemo(() => getPostureCheckTableFilterMessages(), []);

  const { data, fetchNextPage, isFetchingNextPage, isLoading } = useInfiniteQuery({
    queryKey: ['device-posture', requestFilters],
    initialPageParam: 1,
    queryFn: ({ pageParam }) =>
      api.devicePosture.getDevicePosturesPage({
        ...requestFilters,
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
  const postureChecks = useMemo(
    () => flatQueryData?.flatMap((page) => page.data).map(mapApiDevicePostureToRow) ?? [],
    [flatQueryData],
  );
  const lastItem = flatQueryData ? flatQueryData[flatQueryData.length - 1] : null;
  const pagination = lastItem ? lastItem.pagination : null;

  const addButtonProps = useMemo(
    (): ButtonProps => ({
      text: m.posture_checks_button_add(),
      iconLeft: 'plus',
      loading: licenseInfoFetching,
      testId: 'add-posture-check',
      onClick: () => {
        licenseActionCheck(canUseEnterpriseFeature(licenseInfo), () => {
          // TODO: Implement add posture check flow
        });
      },
    }),
    [licenseInfo, licenseInfoFetching],
  );

  if (isLoading) {
    return <TableSkeleton />;
  }

  return (
    <TablePageLayout>
      {postureChecks.length > 0 || columnFilters.length > 0 ? (
        <PostureChecksTable
          addButtonProps={addButtonProps}
          columnFilters={columnFilters}
          filterMessages={filterMessages}
          hasNextPage={pagination?.next_page !== null}
          loadingNextPage={isFetchingNextPage}
          onColumnFiltersChange={setColumnFilters}
          onNextPage={() => {
            fetchNextPage();
          }}
          postureChecks={postureChecks}
        />
      ) : (
        <EmptyStateFlexible
          icon="posture-checks"
          title={m.posture_checks_empty_title()}
          subtitle={m.posture_checks_empty_subtitle()}
          primaryAction={addButtonProps}
        />
      )}
    </TablePageLayout>
  );
};

export const PostureChecksPage = () => {
  return (
    <Page id="posture-checks-page" title={m.cmp_nav_item_posture_checks()}>
      <SizedBox height={ThemeSpacing.Xl3} />
      <Suspense fallback={<TableSkeleton />}>
        <PostureChecksContent />
      </Suspense>
    </Page>
  );
};
