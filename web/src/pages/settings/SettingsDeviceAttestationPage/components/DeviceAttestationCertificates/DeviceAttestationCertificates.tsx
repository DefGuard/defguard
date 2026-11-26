import { keepPreviousData, useInfiniteQuery, useMutation } from '@tanstack/react-query';
import {
  createColumnHelper,
  getCoreRowModel,
  type SortingState,
  useReactTable,
} from '@tanstack/react-table';
import { useMemo, useState } from 'react';
import { m } from '../../../../../paraglide/messages';
import api from '../../../../../shared/api/api';
import type {
  TpmCaCertificate,
  TpmCaCertificateSortKey,
} from '../../../../../shared/api/types';
import { TableSkeleton } from '../../../../../shared/components/skeleton/TableSkeleton/TableSkeleton';
import { EmptyStateFlexible } from '../../../../../shared/defguard-ui/components/EmptyStateFlexible/EmptyStateFlexible';
import { IconButton } from '../../../../../shared/defguard-ui/components/IconButton/IconButton';
import { Search } from '../../../../../shared/defguard-ui/components/Search/Search';
import { tableEditColumnSize } from '../../../../../shared/defguard-ui/components/table/consts';
import { TableBody } from '../../../../../shared/defguard-ui/components/table/TableBody/TableBody';
import { TableCell } from '../../../../../shared/defguard-ui/components/table/TableCell/TableCell';
import { TableTop } from '../../../../../shared/defguard-ui/components/table/TableTop/TableTop';
import { Snackbar } from '../../../../../shared/defguard-ui/providers/snackbar/snackbar';
import { isPresent } from '../../../../../shared/defguard-ui/utils/isPresent';
import { displayDate } from '../../../../../shared/utils/displayDate';
import { downloadFile } from '../../../../../shared/utils/download';
import { formatFileName } from '../../../../../shared/utils/formatFileName';

const columnHelper = createColumnHelper<TpmCaCertificate>();

const downloadCertificatePem = async (certificate: TpmCaCertificate) => {
  const { data } = await api.deviceAttestation.getCertificatePem(certificate.sha256);
  const blob = new Blob([data.pem], {
    type: 'application/x-pem-file;charset=utf-8',
  });
  downloadFile(blob, formatFileName(certificate.common_name), 'pem');
};

export const DeviceAttestationCertificates = () => {
  const [search, setSearch] = useState('');
  const [sortingState, setSortingState] = useState<SortingState>([
    { id: 'name', desc: false },
  ]);
  const activeSorting = sortingState[0];

  const { data, fetchNextPage, isFetchingNextPage, isLoading } = useInfiniteQuery({
    queryKey: ['device-attestation', 'certificates', { search, sortingState }],
    initialPageParam: 1,
    queryFn: ({ pageParam }) =>
      api.deviceAttestation.getCertificates({
        page: pageParam,
        search: search.length > 0 ? search : undefined,
        sort_by: activeSorting?.id as TpmCaCertificateSortKey | undefined,
        sort_order: activeSorting ? (activeSorting.desc ? 'desc' : 'asc') : undefined,
      }),
    placeholderData: keepPreviousData,
    getNextPageParam: (lastPage) => lastPage?.pagination.next_page,
  });

  const certificates = useMemo(
    () => data?.pages.flatMap((page) => page.data) ?? [],
    [data?.pages],
  );
  const lastPage = data?.pages[data.pages.length - 1];
  const hasNextPage = isPresent(lastPage) && lastPage.pagination.next_page !== null;

  const { mutate: downloadPem } = useMutation({
    mutationFn: downloadCertificatePem,
    onError: () => {
      Snackbar.error(m.settings_device_attestation_download_error());
    },
  });

  const columns = useMemo(
    () => [
      columnHelper.accessor('common_name', {
        id: 'name',
        header: m.settings_device_attestation_col_ca(),
        enableSorting: true,
        minSize: 300,
        meta: {
          flex: true,
        },
        cell: (info) => (
          <TableCell>
            <span>{info.getValue()}</span>
          </TableCell>
        ),
      }),
      columnHelper.accessor('not_after', {
        header: m.settings_device_attestation_col_valid_until(),
        enableSorting: true,
        minSize: 180,
        cell: (info) => (
          <TableCell>
            <span>{displayDate(info.getValue())}</span>
          </TableCell>
        ),
      }),
      columnHelper.display({
        id: 'download',
        header: '',
        size: tableEditColumnSize,
        enableResizing: false,
        cell: (info) => (
          <TableCell>
            <IconButton
              icon="download"
              onClick={() => {
                downloadPem(info.row.original);
              }}
            />
          </TableCell>
        ),
      }),
    ],
    [downloadPem],
  );

  const table = useReactTable({
    state: {
      sorting: sortingState,
    },
    data: certificates,
    columns,
    columnResizeMode: 'onChange',
    getCoreRowModel: getCoreRowModel(),
    enableRowSelection: false,
    enableSorting: true,
    manualSorting: true,
    enableSortingRemoval: false,
    onSortingChange: setSortingState,
  });

  if (isLoading) return <TableSkeleton />;

  return (
    <>
      <TableTop text={m.settings_device_attestation_table_title()}>
        <Search placeholder={m.controls_search()} value={search} onChange={setSearch} />
      </TableTop>
      {certificates.length > 0 && (
        <TableBody
          table={table}
          hasNextPage={hasNextPage}
          loadingNextPage={isFetchingNextPage}
          onNextPage={() => {
            fetchNextPage();
          }}
        />
      )}
      {certificates.length === 0 && (
        <EmptyStateFlexible
          icon="search"
          title={m.search_empty_common_title()}
          subtitle={m.settings_device_attestation_search_empty_subtitle()}
          secondaryAction={() => {
            setSearch('');
          }}
          secondaryActionText={m.controls_reset()}
        />
      )}
    </>
  );
};
