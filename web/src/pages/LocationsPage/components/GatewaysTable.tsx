import { useMutation, useSuspenseQuery } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import {
  createColumnHelper,
  getCoreRowModel,
  getSortedRowModel,
  useReactTable,
} from '@tanstack/react-table';
import { useMemo, useState } from 'react';
import { m } from '../../../paraglide/messages';
import api from '../../../shared/api/api';
import type { GatewayInfo } from '../../../shared/api/types';
import { Badge } from '../../../shared/defguard-ui/components/Badge/Badge';
import { EmptyStateFlexible } from '../../../shared/defguard-ui/components/EmptyStateFlexible/EmptyStateFlexible';
import type { MenuItemsGroup } from '../../../shared/defguard-ui/components/Menu/types';
import { Search } from '../../../shared/defguard-ui/components/Search/Search';
import { tableEditColumnSize } from '../../../shared/defguard-ui/components/table/consts';
import { TableBody } from '../../../shared/defguard-ui/components/table/TableBody/TableBody';
import { TableCell } from '../../../shared/defguard-ui/components/table/TableCell/TableCell';
import { TableEditCell } from '../../../shared/defguard-ui/components/table/TableEditCell/TableEditCell';
import { TableTop } from '../../../shared/defguard-ui/components/table/TableTop/TableTop';
import { Snackbar } from '../../../shared/defguard-ui/providers/snackbar/snackbar';
import { openModal } from '../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../shared/hooks/modalControls/modalTypes';
import { getGatewaysQueryOptions } from '../../../shared/query';
import { displayDate } from '../../../shared/utils/displayDate';

type RowData = GatewayInfo;

const columnHelper = createColumnHelper<RowData>();

const getStatusBadge = (gateway: GatewayInfo) => {
  if (!gateway.enabled) {
    return (
      <Badge icon="disabled" showIcon variant="critical" text={m.state_disabled()} />
    );
  }
  if (gateway.connected) {
    return <Badge icon="check-filled" showIcon variant="success" text="Connected" />;
  }

  if (!gateway.connected_at) {
    return (
      <Badge icon="status-attention" showIcon variant="warning" text="Not connected" />
    );
  }

  return (
    <Badge icon="status-important" showIcon variant="critical" text="Disconnected" />
  );
};

export const GatewaysTable = () => {
  const { data: gateways } = useSuspenseQuery(getGatewaysQueryOptions);
  const navigate = useNavigate();
  const { mutate: toggleGateway } = useMutation({
    mutationFn: api.gateway.editGateway,
    meta: {
      invalidate: ['gateway'],
    },
  });

  const [search, setSearch] = useState('');

  const transformedData = useMemo(() => {
    let data = gateways;
    const query = search.trim().toLowerCase();

    if (query.length > 0) {
      data = data.filter((gateway) => {
        const modifiedBy = gateway.modified_by.toLowerCase();
        return (
          gateway.name.toLowerCase().includes(query) ||
          gateway.location_name.toLowerCase().includes(query) ||
          modifiedBy.includes(query)
        );
      });
    }

    return data;
  }, [gateways, search]);

  const columns = useMemo(
    () => [
      columnHelper.accessor('name', {
        header: m.edges_col_name(),
        enableSorting: true,
        sortingFn: 'text',
        minSize: 250,
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
      columnHelper.accessor('address', {
        header: m.edges_col_address(),
        size: 175,
        minSize: 175,
        cell: (info) => (
          <TableCell>
            <span>{info.getValue() ?? ''}</span>
          </TableCell>
        ),
      }),
      columnHelper.accessor('port', {
        header: m.edges_col_port(),
        size: 170,
        minSize: 100,
        sortingFn: 'text',
        cell: (info) => (
          <TableCell>
            <span>{info.getValue() ?? ''}</span>
          </TableCell>
        ),
      }),
      columnHelper.accessor('version', {
        size: 175,
        minSize: 175,
        header: m.edges_col_version(),
        enableSorting: true,
        cell: (info) => (
          <TableCell>
            <span>{info.getValue() ?? ''}</span>
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
        size: 175,
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
      columnHelper.accessor('location_name', {
        header: 'Used in location',
        size: 220,
        minSize: 200,
        meta: {
          flex: true,
        },
        cell: (info) => (
          <TableCell>
            <span>{info.getValue()}</span>
          </TableCell>
        ),
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
                  text: m.controls_edit(),
                  icon: 'edit',
                  onClick: () => {
                    navigate({
                      to: '/gateway/$gatewayId/edit',
                      params: {
                        gatewayId: rowData.id.toString(),
                      },
                    });
                  },
                },
                {
                  text: rowData.enabled ? m.controls_disable() : m.controls_enable(),
                  icon: rowData.enabled ? 'disabled' : 'check-circle',
                  onClick: () => {
                    toggleGateway({
                      id: rowData.id,
                      name: rowData.name,
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
                      title: m.modal_delete_gateway_title(),
                      contentMd: m.modal_delete_gateway_body({
                        name: rowData.name,
                        locationName: rowData.location_name,
                      }),
                      actionPromise: () => api.gateway.deleteGateway(rowData.id),
                      invalidateKeys: [['gateway'], ['network']],
                      submitProps: { text: m.controls_delete(), variant: 'critical' },
                      onSuccess: () => Snackbar.default(m.gateway_delete_success()),
                      onError: () => Snackbar.error(m.gateway_delete_failed()),
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
    [navigate, toggleGateway],
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

  if (gateways.length === 0) {
    return <EmptyStateFlexible title="No gateways found" />;
  }

  return (
    <>
      <TableTop text="Gateways management">
        <Search
          placeholder={m.controls_search()}
          initialValue={search}
          onChange={setSearch}
        />
      </TableTop>
      {transformedData.length === 0 && search.length > 0 && (
        <EmptyStateFlexible
          icon="search"
          title={m.search_empty_common_title()}
          subtitle={m.search_empty_common_subtitle()}
        />
      )}
      {transformedData.length > 0 && <TableBody table={table} />}
    </>
  );
};
