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
import type { EdgeInfo } from '../../shared/api/types';
import { Badge } from '../../shared/defguard-ui/components/Badge/Badge';
import { Button } from '../../shared/defguard-ui/components/Button/Button';
import type { ButtonProps } from '../../shared/defguard-ui/components/Button/types';
import { EmptyState } from '../../shared/defguard-ui/components/EmptyState/EmptyState';
import { EmptyStateFlexible } from '../../shared/defguard-ui/components/EmptyStateFlexible/EmptyStateFlexible';
import { IconButtonMenu } from '../../shared/defguard-ui/components/IconButtonMenu/IconButtonMenu';
import type { MenuItemsGroup } from '../../shared/defguard-ui/components/Menu/types';
import { Search } from '../../shared/defguard-ui/components/Search/Search';
import { tableEditColumnSize } from '../../shared/defguard-ui/components/table/consts';
import { TableBody } from '../../shared/defguard-ui/components/table/TableBody/TableBody';
import { TableCell } from '../../shared/defguard-ui/components/table/TableCell/TableCell';
import { TableTop } from '../../shared/defguard-ui/components/table/TableTop/TableTop';
import { isPresent } from '../../shared/defguard-ui/utils/isPresent';
import { displayDate } from '../../shared/utils/displayDate';

type Props = {
  edges: EdgeInfo[];
};

type RowData = EdgeInfo;

const columnHelper = createColumnHelper<RowData>();

const isConnected = (edge: EdgeInfo) => {
  if (!isPresent(edge.connected_at)) return false;

  if (!isPresent(edge.disconnected_at)) return true;

  const connected = dayjs.utc(edge.connected_at);
  const disconnected = dayjs.utc(edge.disconnected_at);

  return connected > disconnected;
};

const displayModifiedBy = (edge: EdgeInfo) =>
  `${edge.modified_by_firstname} ${edge.modified_by_lastname}`;

export const EdgesTable = ({ edges }: Props) => {
  const navigate = useNavigate();

  const addButtonProps = useMemo(
    (): ButtonProps => ({
      variant: 'primary',
      text: 'Add Edge component',
      iconLeft: 'globe',
      testId: 'add-edge',
      onClick: () => {
        navigate({ to: '/setup-edge' });
      },
    }),
    [navigate],
  );

  const [search, setSearch] = useState('');

  const transformedData = useMemo(() => {
    let data = edges;
    if (search.length) {
      data = data.filter((u) => u.name.toLowerCase().includes(search.toLowerCase()));
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
        cell: (info) => (
          <TableCell>
            <span>{info.getValue()}</span>
          </TableCell>
        ),
      }),
      columnHelper.accessor('address', {
        header: m.edges_col_address(),
        size: 175,
        minSize: 175,
        cell: (info) => (
          <TableCell>
            <span>{info.getValue()}</span>
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
            <span>{info.getValue()}</span>
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
      columnHelper.display({
        id: 'status',
        size: 175,
        minSize: 175,
        header: m.edges_col_status(),
        enableSorting: false,
        cell: (info) => (
          <TableCell>
            {isConnected(info.row.original) && (
              <Badge
                icon="check-filled"
                showIcon
                variant="success"
                text={m.edge_connected()}
              />
            )}
            {!isConnected(info.row.original) && (
              <Badge
                icon="status-important"
                showIcon
                variant="critical"
                text={m.edge_disconnected()}
              />
            )}
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
                  text: m.edges_row_menu_edit(),
                  icon: 'edit',
                  onClick: () => {
                    navigate({
                      to: `/edge/$edgeId/edit`,
                      params: { edgeId: rowData.id.toString() },
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
    [navigate],
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
