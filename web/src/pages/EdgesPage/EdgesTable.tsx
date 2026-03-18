import { useMutation, useSuspenseQuery } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import {
  createColumnHelper,
  getCoreRowModel,
  getSortedRowModel,
  useReactTable,
} from '@tanstack/react-table';
import dayjs from 'dayjs';
import { useMemo, useState } from 'react';
import { m } from '../../paraglide/messages';
import api from '../../shared/api/api';
import type { EdgeInfo } from '../../shared/api/types';
import { Badge } from '../../shared/defguard-ui/components/Badge/Badge';
import { Button } from '../../shared/defguard-ui/components/Button/Button';
import type { ButtonProps } from '../../shared/defguard-ui/components/Button/types';
import { EmptyState } from '../../shared/defguard-ui/components/EmptyState/EmptyState';
import { EmptyStateFlexible } from '../../shared/defguard-ui/components/EmptyStateFlexible/EmptyStateFlexible';
import type { MenuItemsGroup } from '../../shared/defguard-ui/components/Menu/types';
import { Search } from '../../shared/defguard-ui/components/Search/Search';
import { tableEditColumnSize } from '../../shared/defguard-ui/components/table/consts';
import { TableBody } from '../../shared/defguard-ui/components/table/TableBody/TableBody';
import { TableCell } from '../../shared/defguard-ui/components/table/TableCell/TableCell';
import { TableEditCell } from '../../shared/defguard-ui/components/table/TableEditCell/TableEditCell';
import { TableTop } from '../../shared/defguard-ui/components/table/TableTop/TableTop';
import { isPresent } from '../../shared/defguard-ui/utils/isPresent';
import { Snackbar } from '../../shared/defguard-ui/providers/snackbar/snackbar';
import { openModal } from '../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../shared/hooks/modalControls/modalTypes';
import { getEdgesQueryOptions, getLicenseInfoQueryOptions } from '../../shared/query';
import { displayDate } from '../../shared/utils/displayDate';
import { canUseEnterpriseFeature, licenseActionCheck } from '../../shared/utils/license';

type RowData = EdgeInfo;

const columnHelper = createColumnHelper<RowData>();

const isConnected = (edge: EdgeInfo) => {
  if (!isPresent(edge.connected_at)) return false;

  if (!isPresent(edge.disconnected_at)) return true;

  const connected = dayjs.utc(edge.connected_at);
  const disconnected = dayjs.utc(edge.disconnected_at);

  return connected > disconnected;
};

const getStatusBadge = (edge: EdgeInfo) => {
  if (!edge.enabled) {
    return (
      <Badge icon="disabled" showIcon variant="critical" text={m.state_disabled()} />
    );
  }

  if (isConnected(edge)) {
    return (
      <Badge icon="check-filled" showIcon variant="success" text={m.edge_connected()} />
    );
  }

  return (
    <Badge
      icon="status-important"
      showIcon
      variant="critical"
      text={m.edge_disconnected()}
    />
  );
};

export const EdgesTable = () => {
  const { data: edges } = useSuspenseQuery(getEdgesQueryOptions);
  const { data: licenseInfo } = useSuspenseQuery(getLicenseInfoQueryOptions);

  const navigate = useNavigate();

  const { mutate: toggleEdge } = useMutation({
    mutationFn: api.edge.editEdge,
    meta: {
      invalidate: ['edge'],
    },
  });

  const addButtonProps = useMemo(
    (): ButtonProps => ({
      variant: 'primary',
      text: m.edge_add(),
      iconLeft: 'globe',
      testId: 'add-edge',
      onClick: () => {
        if (edges.length >= 1) {
          licenseActionCheck(canUseEnterpriseFeature(licenseInfo), () => {
            navigate({ to: '/setup-edge' });
          });
        } else {
          navigate({ to: '/setup-edge' });
        }
      },
    }),
    [navigate, edges.length, licenseInfo],
  );

  const [search, setSearch] = useState('');

  const transformedData = useMemo(() => {
    let data = edges;
    if (search.length) {
      const query = search.toLowerCase();
      data = data.filter(
        (edge) =>
          edge.name.toLowerCase().includes(query) ||
          edge.modified_by.toLowerCase().includes(query),
      );
    }

    return data;
  }, [edges, search.length, search.toLowerCase, search]);

  const columns = useMemo(
    () => [
      columnHelper.accessor('name', {
        header: m.edges_col_name(),
        enableSorting: true,
        sortingFn: 'text',
        minSize: 250,
        size: 300,
        cell: (info) => (
          <TableCell>
            <span>{info.getValue()}</span>
          </TableCell>
        ),
      }),
      columnHelper.accessor('address', {
        header: m.edges_col_address(),
        minSize: 250,
        meta: {
          flex: true,
        },
        cell: (info) => (
          <TableCell>
            <span>{info.getValue()}</span>
          </TableCell>
        ),
      }),
      columnHelper.accessor('port', {
        header: m.edges_col_port(),
        minSize: 125,
        sortingFn: 'text',
        cell: (info) => (
          <TableCell>
            <span>{info.getValue()}</span>
          </TableCell>
        ),
      }),
      columnHelper.accessor('version', {
        minSize: 125,
        header: m.edges_col_version(),
        enableSorting: true,
        cell: (info) => (
          <TableCell>
            <span>{info.getValue()}</span>
          </TableCell>
        ),
      }),
      columnHelper.accessor('modified_at', {
        size: 175,
        minSize: 175,
        header: m.edges_col_last_modified(),
        enableSorting: true,
        cell: (info) => (
          <TableCell>
            <span>{displayDate(info.getValue())}</span>
          </TableCell>
        ),
      }),
      columnHelper.accessor('modified_by', {
        size: 200,
        minSize: 175,
        header: m.edges_col_modified_by(),
        enableSorting: true,
        sortingFn: 'text',
        cell: (info) => (
          <TableCell>
            <span>{info.getValue()}</span>
          </TableCell>
        ),
      }),
      columnHelper.display({
        id: 'status',
        size: 175,
        minSize: 175,
        header: m.edges_col_status(),
        enableSorting: false,
        cell: (info) => <TableCell>{getStatusBadge(info.row.original)}</TableCell>,
      }),
      columnHelper.display({
        id: 'edit',
        size: tableEditColumnSize,
        header: '',
        enableSorting: false,
        enableResizing: false,
        cell: (info) => {
          const rowData = info.row.original;

          const menuItems: MenuItemsGroup[] = [
            {
              items: [
                {
                  text: m.edges_row_menu_edit(),
                  icon: 'edit',
                  onClick: () => {
                    navigate({
                      to: `/edge/$edgeId/edit`,
                      params: { edgeId: rowData.id.toString() },
                    });
                  },
                },
                {
                  text: rowData.enabled ? m.controls_disable() : m.controls_enable(),
                  icon: rowData.enabled ? 'disabled' : 'check-circle',
                  onClick: () => {
                    toggleEdge({
                      ...rowData,
                      enabled: !rowData.enabled,
                    });
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
                    openModal(ModalName.ConfirmAction, {
                      title: m.modal_delete_edge_title(),
                      contentMd: m.modal_delete_edge_body({ name: rowData.name }),
                      actionPromise: () => api.edge.deleteEdge(rowData.id),
                      invalidateKeys: [['edge']],
                      submitProps: { text: m.controls_delete(), variant: 'critical' },
                      onSuccess: () => Snackbar.default(m.modal_delete_edge_success()),
                      onError: () => Snackbar.error(m.modal_delete_edge_error()),
                    });
                  },
                },
              ],
            },
          ];

          return <TableEditCell menuItems={menuItems} />;
        },
      }),
    ],
    [navigate, toggleEdge],
  );

  const table = useReactTable({
    initialState: {
      sorting: [
        {
          id: 'name',
          desc: false,
        },
      ],
    },
    columns,
    data: transformedData,
    enableRowSelection: false,
    columnResizeMode: 'onChange',
    getSortedRowModel: getSortedRowModel(),
    getCoreRowModel: getCoreRowModel(),
  });

  if (edges.length === 0)
    return (
      <EmptyStateFlexible
        title={m.edges_empty_title()}
        subtitle={m.edges_empty_subtitle()}
        primaryAction={addButtonProps}
      />
    );

  return (
    <>
      <TableTop text={m.edges_header_title()}>
        <Search
          placeholder={m.edges_search_placeholder()}
          initialValue={search}
          onChange={setSearch}
        />
        <Button {...addButtonProps} />
      </TableTop>
      {edges.length === 0 && search.length > 0 && (
        <EmptyState
          icon="search"
          title={m.search_empty_common_title()}
          subtitle={m.search_empty_common_subtitle()}
        />
      )}
      {edges.length > 0 && <TableBody table={table} />}
    </>
  );
};
