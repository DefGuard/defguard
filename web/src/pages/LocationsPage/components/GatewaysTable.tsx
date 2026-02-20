import { useSuspenseQuery } from '@tanstack/react-query';
import {
  createColumnHelper,
  getCoreRowModel,
  getSortedRowModel,
  useReactTable,
} from '@tanstack/react-table';
import { useMemo, useState } from 'react';
import { m } from '../../../paraglide/messages';
import type { GatewayInfo } from '../../../shared/api/types';
import { Badge } from '../../../shared/defguard-ui/components/Badge/Badge';
import { EmptyStateFlexible } from '../../../shared/defguard-ui/components/EmptyStateFlexible/EmptyStateFlexible';
import { IconButtonMenu } from '../../../shared/defguard-ui/components/IconButtonMenu/IconButtonMenu';
import type { MenuItemsGroup } from '../../../shared/defguard-ui/components/Menu/types';
import { Search } from '../../../shared/defguard-ui/components/Search/Search';
import { tableEditColumnSize } from '../../../shared/defguard-ui/components/table/consts';
import { TableBody } from '../../../shared/defguard-ui/components/table/TableBody/TableBody';
import { TableCell } from '../../../shared/defguard-ui/components/table/TableCell/TableCell';
import { TableTop } from '../../../shared/defguard-ui/components/table/TableTop/TableTop';
import { openModal } from '../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../shared/hooks/modalControls/modalTypes';
import { getGatewaysQueryOptions } from '../../../shared/query';
import { displayDate } from '../../../shared/utils/displayDate';

type RowData = GatewayInfo;

const columnHelper = createColumnHelper<RowData>();

const displayModifiedBy = (gateway: GatewayInfo) =>
  `${gateway.modified_by_firstname} ${gateway.modified_by_lastname}`;

const getStatusBadge = (gateway: GatewayInfo) => {
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

  const [search, setSearch] = useState('');

  const transformedData = useMemo(() => {
    let data = gateways;
    const query = search.trim().toLowerCase();

    if (query.length > 0) {
      data = data.filter((gateway) => {
        const modifiedBy = displayModifiedBy(gateway).toLowerCase();
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
      columnHelper.display({
        id: 'modified_by',
        size: 175,
        minSize: 175,
        header: m.edges_col_modified_by(),
        enableSorting: true,
        cell: (info) => (
          <TableCell>
            <span>{displayModifiedBy(info.row.original)}</span>
          </TableCell>
        ),
      }),
      columnHelper.accessor('location_name', {
        header: 'Used in location',
        size: 220,
        minSize: 200,
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
                  text: m.controls_delete(),
                  icon: 'delete',
                  variant: 'danger',
                  onClick: () => {
                    openModal(ModalName.DeleteGateway, {
                      id: rowData.id,
                      name: rowData.name,
                      locationName: rowData.location_name,
                    });
                  },
                },
              ],
            },
          ];

          return (
            <TableCell>
              <IconButtonMenu icon="menu" menuItems={menuItems} />
            </TableCell>
          );
        },
      }),
    ],
    [],
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
