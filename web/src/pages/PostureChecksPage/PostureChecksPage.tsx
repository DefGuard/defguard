import {
  useInfiniteQuery,
  useQuery,
  useQueryClient,
  useSuspenseQuery,
} from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import type { ColumnFiltersState } from '@tanstack/react-table';
import { Suspense, useCallback, useMemo, useState } from 'react';
import { m } from '../../paraglide/messages';
import api from '../../shared/api/api';
import { LicenseFeature } from '../../shared/api/types';
import { Page } from '../../shared/components/Page/Page';
import { TableSkeleton } from '../../shared/components/skeleton/TableSkeleton/TableSkeleton';
import type { ButtonProps } from '../../shared/defguard-ui/components/Button/types';
import { EmptyStateFlexible } from '../../shared/defguard-ui/components/EmptyStateFlexible/EmptyStateFlexible';
import { SizedBox } from '../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../shared/defguard-ui/types';
import { TablePageLayout } from '../../shared/layout/TablePageLayout/TablePageLayout';
import {
  getDevicePostureVersionMetadataQueryOptions,
  getLicenseInfoQueryOptions,
} from '../../shared/query';
import { canUseEnterpriseFeature, licenseActionCheck } from '../../shared/utils/license';
import { shouldFetchPostureChecksEnterpriseData } from './license';
import { PostureCheckDrawer } from './PostureCheckDrawer/PostureCheckDrawer';
import { PostureChecksTable } from './PostureChecksTable';
import {
  getPostureCheckColumnFilterOptions,
  getPostureCheckTableFilterMessages,
  mapApiDevicePostureToRow,
  mapPostureCheckFilterValueToRequestValue,
  type PostureCheckRow,
} from './postureChecks';
import { getPostureCheckVersionValues } from './types';

const mapColumnFiltersToRequest = (columnFilters: ColumnFiltersState) => {
  const result: Record<string, string[]> = {};

  for (const filter of columnFilters) {
    if (Array.isArray(filter.value) && filter.value.length > 0) {
      result[filter.id] = filter.value
        .filter(
          (value): value is string | number =>
            typeof value === 'string' || typeof value === 'number',
        )
        .map(mapPostureCheckFilterValueToRequestValue);
    }
  }

  return result;
};

const PostureChecksContent = () => {
  const navigate = useNavigate();
  const { data: licenseInfo, isFetching: licenseInfoFetching } = useSuspenseQuery(
    getLicenseInfoQueryOptions,
  );
  const canUseEnterprise = useMemo(
    () => canUseEnterpriseFeature(licenseInfo, LicenseFeature.DevicePosture).result,
    [licenseInfo],
  );
  const { data: versionMetadata, isLoading: versionMetadataLoading } = useQuery({
    ...getDevicePostureVersionMetadataQueryOptions,
    enabled: shouldFetchPostureChecksEnterpriseData(canUseEnterprise),
  });
  const [columnFilters, setColumnFilters] = useState<ColumnFiltersState>([]);
  const [selectedRow, setSelectedRow] = useState<PostureCheckRow | null>(null);
  const queryClient = useQueryClient();
  const handleDrawerClose = useCallback(() => {
    setSelectedRow(null);
  }, []);
  const versionValues = useMemo(
    () => (versionMetadata ? getPostureCheckVersionValues(versionMetadata) : null),
    [versionMetadata],
  );
  const columnFilterOptions = useMemo(
    () =>
      versionValues
        ? getPostureCheckColumnFilterOptions(versionValues)
        : {
            windows: [],
            macos: [],
            linux: [],
            ios: [],
            android: [],
            defguard_desktop: [],
            defguard_mobile: [],
            defguard: [],
          },
    [versionValues],
  );
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
    enabled: shouldFetchPostureChecksEnterpriseData(canUseEnterprise),
  });

  const flatQueryData = useMemo(() => data?.pages.flat() ?? null, [data?.pages]);
  const flatPostures = useMemo(
    () => flatQueryData?.flatMap((page) => page.data) ?? [],
    [flatQueryData],
  );
  const postureChecks = useMemo(
    () => flatPostures.map(mapApiDevicePostureToRow),
    [flatPostures],
  );
  const posturesById = useMemo(
    () => new Map(flatPostures.map((posture) => [posture.id, posture])),
    [flatPostures],
  );
  const handleRowClick = useCallback(
    (row: PostureCheckRow) => {
      const posture = posturesById.get(row.id);
      if (posture) {
        queryClient.setQueryData(['device-posture', row.id], posture);
      }
      setSelectedRow(row);
    },
    [posturesById, queryClient],
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
        licenseActionCheck(
          canUseEnterpriseFeature(licenseInfo, LicenseFeature.DevicePosture),
          () => {
            void navigate({ to: '/add-posture-check' });
          },
        );
      },
    }),
    [licenseInfo, licenseInfoFetching, navigate],
  );

  if (canUseEnterprise && (isLoading || versionMetadataLoading)) {
    return <TableSkeleton />;
  }

  return (
    <>
      <TablePageLayout>
        {postureChecks.length > 0 || columnFilters.length > 0 ? (
          <PostureChecksTable
            addButtonProps={addButtonProps}
            columnFilterOptions={columnFilterOptions}
            columnFilters={columnFilters}
            filterMessages={filterMessages}
            hasNextPage={pagination?.next_page !== null}
            loadingNextPage={isFetchingNextPage}
            onColumnFiltersChange={setColumnFilters}
            onNextPage={() => {
              fetchNextPage();
            }}
            onRowClick={handleRowClick}
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
      <PostureCheckDrawer selectedRow={selectedRow} onClose={handleDrawerClose} />
    </>
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
