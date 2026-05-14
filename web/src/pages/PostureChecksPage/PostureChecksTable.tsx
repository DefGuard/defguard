import { Link } from '@tanstack/react-router';
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
import { Button } from '../../shared/defguard-ui/components/Button/Button';
import type { ButtonProps } from '../../shared/defguard-ui/components/Button/types';
import { EmptyStateFlexible } from '../../shared/defguard-ui/components/EmptyStateFlexible/EmptyStateFlexible';
import { IconKind } from '../../shared/defguard-ui/components/Icon';
import type { MenuItemsGroup } from '../../shared/defguard-ui/components/Menu/types';
import { tableEditColumnSize } from '../../shared/defguard-ui/components/table/consts';
import { TableBody } from '../../shared/defguard-ui/components/table/TableBody/TableBody';
import { TableCell } from '../../shared/defguard-ui/components/table/TableCell/TableCell';
import { TableEditCell } from '../../shared/defguard-ui/components/table/TableEditCell/TableEditCell';
import { TableTop } from '../../shared/defguard-ui/components/table/TableTop/TableTop';
import type { TableFilterMessages } from '../../shared/defguard-ui/components/table/types';
import { Snackbar } from '../../shared/defguard-ui/providers/snackbar/snackbar';
import type { PostureCheckColumnFilterOptions, PostureCheckRow } from './postureChecks';
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
  postureChecks,
}: Props) => {
  const columns = useMemo(
    () => [
      columnHelper.accessor('name', {
        header: 'Title',
        minSize: 306,
        cell: (info) => (
          <TableCell>
            <Link to="." className="posture-check-link">
              {info.getValue()}
            </Link>
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
        header: 'MacOS',
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
          const menuItems: MenuItemsGroup[] = [
            {
              items: [
                {
                  text: m.controls_edit(),
                  icon: 'edit',
                  onClick: () => {
                    Snackbar.default(`Edit is not available yet for "${row.name}".`);
                  },
                },
                {
                  text: 'Duplicate',
                  icon: IconKind.Duplicate,
                  onClick: () => {
                    Snackbar.default(`Duplicate is not available yet for "${row.name}".`);
                  },
                },
                {
                  text: 'Assign to location',
                  icon: 'add-location',
                  onClick: () => {
                    Snackbar.default(
                      `Location assignment is not available yet for "${row.name}".`,
                    );
                  },
                },
              ],
            },
            {
              items: [
                {
                  text: m.controls_delete(),
                  icon: 'delete',
                  variant: 'danger',
                  onClick: () => {
                    Snackbar.default(`Delete is not available yet for "${row.name}".`);
                  },
                },
              ],
            },
          ];

          return <TableEditCell menuItems={menuItems} />;
        },
      }),
    ],
    [columnFilterOptions],
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
