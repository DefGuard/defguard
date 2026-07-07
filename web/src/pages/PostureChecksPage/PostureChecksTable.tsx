import { useMutation, useSuspenseQuery } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import {
  type ColumnFiltersState,
  createColumnHelper,
  getCoreRowModel,
  getFilteredRowModel,
  type OnChangeFn,
  useReactTable,
} from '@tanstack/react-table';
import { useMemo } from 'react';
import { m } from '../../paraglide/messages';
import api from '../../shared/api/api';
import { Button } from '../../shared/defguard-ui/components/Button/Button';
import type { ButtonProps } from '../../shared/defguard-ui/components/Button/types';
import { EmptyStateFlexible } from '../../shared/defguard-ui/components/EmptyStateFlexible/EmptyStateFlexible';
import { tableEditColumnSize } from '../../shared/defguard-ui/components/table/consts';
import { TableBody } from '../../shared/defguard-ui/components/table/TableBody/TableBody';
import { TableCell } from '../../shared/defguard-ui/components/table/TableCell/TableCell';
import { TableEditCell } from '../../shared/defguard-ui/components/table/TableEditCell/TableEditCell';
import { TableTop } from '../../shared/defguard-ui/components/table/TableTop/TableTop';
import type { TableFilterMessages } from '../../shared/defguard-ui/components/table/types';
import { Snackbar } from '../../shared/defguard-ui/providers/snackbar/snackbar';
import { getLocationsQueryOptions } from '../../shared/query';
import { buildPostureCheckMenuItems } from './postureCheckMenu';
import {
  buildFilteredLocationOptions,
  type PostureCheckColumnFilterOptions,
  type PostureCheckRow,
} from './postureChecks';
import './style.scss';

type Props = {
  addButtonProps: ButtonProps;
  columnFilterOptions: PostureCheckColumnFilterOptions;
  columnFilters: ColumnFiltersState;
  filterMessages: TableFilterMessages;
  hasNextPage: boolean;
  loadingNextPage: boolean;
  onColumnFiltersChange: OnChangeFn<ColumnFiltersState>;
  onNextPage: () => void;
  onRowClick: (row: PostureCheckRow) => void;
  postureChecks: PostureCheckRow[];
};

const columnHelper = createColumnHelper<PostureCheckRow>();

export const PostureChecksTable = ({
  addButtonProps,
  columnFilterOptions,
  columnFilters,
  filterMessages,
  hasNextPage,
  loadingNextPage,
  onColumnFiltersChange,
  onNextPage,
  onRowClick,
  postureChecks,
}: Props) => {
  const navigate = useNavigate();
  const { data: locations } = useSuspenseQuery(getLocationsQueryOptions);
  const locationOptions = useMemo(
    () => buildFilteredLocationOptions(locations),
    [locations],
  );
  const { mutate: assignLocations } = useMutation({
    mutationFn: ({
      postureCheckId,
      locations,
    }: {
      postureCheckId: number;
      locations: number[];
    }) => api.devicePosture.setLocationsForDevicePosture(postureCheckId, locations),
    meta: {
      invalidate: [['device-posture'], ['network'], ['activity-log']],
    },
    onSuccess: () => {
      Snackbar.default(m.modal_assign_posture_check_locations_success());
    },
    onError: () => {
      Snackbar.error(m.modal_assign_posture_check_locations_error());
    },
  });

  const { mutate: duplicatePosture } = useMutation({
    mutationFn: api.devicePosture.duplicateDevicePosture,
    meta: {
      invalidate: [['device-posture'], ['network'], ['activity-log']],
    },
    onSuccess: (response) => {
      const posture = response.data;
      Snackbar.default(m.posture_checks_duplication_success());
      navigate({
        to: '/acl/posture-checks/$postureCheckId/edit',
        params: {
          postureCheckId: String(posture.id),
        },
      });
    },
    onError: () => {
      Snackbar.error(m.posture_checks_duplication_failed());
    },
  });
  const columns = useMemo(
    () => [
      columnHelper.accessor('name', {
        header: 'Title',
        minSize: 306,
        cell: (info) => (
          <TableCell>
            <button
              className="posture-check-link"
              onClick={() => onRowClick(info.row.original)}
            >
              {info.getValue()}
            </button>
          </TableCell>
        ),
      }),
      columnHelper.accessor('windowsFilters', {
        id: 'windows',
        header: 'Windows',
        minSize: 180,
        enableColumnFilter: true,
        filterFn: 'arrIncludesSome',
        meta: {
          filterOptions: columnFilterOptions.windows,
        },
        cell: (info) => (
          <TableCell>
            <span>{info.row.original.windows}</span>
          </TableCell>
        ),
      }),
      columnHelper.accessor('macosFilters', {
        id: 'macos',
        header: 'macOS',
        minSize: 180,
        enableColumnFilter: true,
        filterFn: 'arrIncludesSome',
        meta: {
          filterOptions: columnFilterOptions.macos,
        },
        cell: (info) => (
          <TableCell>
            <span>{info.row.original.macos}</span>
          </TableCell>
        ),
      }),
      columnHelper.accessor('linuxFilters', {
        id: 'linux',
        header: 'Linux',
        minSize: 180,
        enableColumnFilter: true,
        filterFn: 'arrIncludesSome',
        meta: {
          filterOptions: columnFilterOptions.linux,
        },
        cell: (info) => (
          <TableCell>
            <span>{info.row.original.linux}</span>
          </TableCell>
        ),
      }),
      columnHelper.accessor('iosFilters', {
        id: 'ios',
        header: 'iOS',
        minSize: 180,
        enableColumnFilter: true,
        filterFn: 'arrIncludesSome',
        meta: {
          filterOptions: columnFilterOptions.ios,
        },
        cell: (info) => (
          <TableCell>
            <span>{info.row.original.ios}</span>
          </TableCell>
        ),
      }),
      columnHelper.accessor('androidFilters', {
        id: 'android',
        header: 'Android',
        minSize: 180,
        enableColumnFilter: true,
        filterFn: 'arrIncludesSome',
        meta: {
          filterOptions: columnFilterOptions.android,
        },
        cell: (info) => (
          <TableCell>
            <span>{info.row.original.android}</span>
          </TableCell>
        ),
      }),
      columnHelper.accessor('defguardFilters', {
        id: 'defguard',
        header: 'Defguard',
        minSize: 180,
        enableColumnFilter: true,
        filterFn: 'arrIncludesSome',
        meta: {
          filterOptions: columnFilterOptions.defguard,
        },
        cell: (info) => (
          <TableCell>
            <span>{info.row.original.defguard}</span>
          </TableCell>
        ),
      }),
      columnHelper.display({
        id: 'edit',
        header: '',
        size: tableEditColumnSize,
        enableResizing: false,
        cell: (info) => {
          const row = info.row.original;
          const menuItems = buildPostureCheckMenuItems({
            row,
            locationOptions,
            navigate,
            assignLocations: (locations) =>
              assignLocations({ postureCheckId: row.id, locations }),
            duplicatePosture: () => duplicatePosture(row.id),
          });

          return <TableEditCell menuItems={menuItems} />;
        },
      }),
    ],
    [
      assignLocations,
      columnFilterOptions,
      locationOptions,
      navigate,
      onRowClick,
      duplicatePosture,
    ],
  );

  const table = useReactTable({
    state: {
      columnFilters,
    },
    meta: {
      filterMessages,
    },
    columns,
    data: postureChecks,
    enableRowSelection: false,
    columnResizeMode: 'onChange',
    onColumnFiltersChange,
    getFilteredRowModel: getFilteredRowModel(),
    getCoreRowModel: getCoreRowModel(),
  });

  const rows = table.getRowModel().rows;

  return (
    <>
      <TableTop text="Active posture checks">
        <Button {...addButtonProps} />
      </TableTop>
      <TableBody
        table={table}
        className="posture-checks-table"
        hasNextPage={hasNextPage}
        loadingNextPage={loadingNextPage}
        onNextPage={onNextPage}
      />
      {rows.length === 0 && columnFilters.length > 0 && (
        <EmptyStateFlexible
          icon="search"
          title={m.search_empty_common_title()}
          subtitle={m.search_empty_common_subtitle()}
        />
      )}
    </>
  );
};
