import { useNavigate } from '@tanstack/react-router';
import {
  type ColumnFiltersState,
  createColumnHelper,
  getCoreRowModel,
  getExpandedRowModel,
  getFilteredRowModel,
  getSortedRowModel,
  type RowSelectionState,
  useReactTable,
} from '@tanstack/react-table';
import { useMemo, useState } from 'react';
import { m } from '../../paraglide/messages';
import type { Edge } from '../../shared/api/types';
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

type Props = {
  edges: Edge[];
};

type RowData = Edge;

const columnHelper = createColumnHelper<RowData>();

export const EdgesTable = ({ edges }: Props) => {
  // const appInfo = useApp((s) => s.appInfo);
  const navigate = useNavigate();

  const addButtonProps = useMemo(
    (): ButtonProps => ({
      variant: 'primary',
      text: 'Add Edge component',
      // TODO(jck)
      iconLeft: 'add-user',
      testId: 'add-edge',
      onClick: () => {
        navigate({ to: '/edge-wizard' });
      },
    }),
    [navigate],
  );

  const [search, setSearch] = useState('');
  const [columnFilters, setColumnFilters] = useState<ColumnFiltersState>([]);

  // const { data: groups } = useQuery(getGroupsInfoQueryOptions);

  // const groupsOptions = useMemo(
  //   (): SelectionOption<string>[] =>
  //     groups?.map((g) => ({
  //       id: g.name,
  //       label: g.name,
  //     })) ?? [],
  //   [groups?.map],
  // );

  // const { mutate: deleteUser } = useMutation({
  //   mutationFn: api.user.deleteUser,
  //   meta: {
  //     invalidate: ['user'],
  //   },
  // });

  // const { mutate: changeAccountActiveState } = useMutation({
  //   mutationFn: api.user.activeStateChange,
  //   meta: {
  //     invalidate: ['user'],
  //   },
  // });

  // const { mutate: editUser } = useMutation({
  //   mutationFn: api.user.editUser,
  //   meta: {
  //     invalidate: ['user'],
  //   },
  // });

  // const handleEditGroups = useCallback(
  //   async (user: RowData, groups: string[]) => {
  //     const freshUser = (await api.user.getUser(user.username)).data.user;
  //     freshUser.groups = groups;
  //     editUser({
  //       username: freshUser.username,
  //       body: freshUser,
  //     });
  //   },
  //   [editUser],
  // );

  const [selected, setSelected] = useState<RowSelectionState>({});

  const transformedData = useMemo(() => {
    let data = edges;
    if (search.length) {
      data = data.filter(
        (u) =>
          u.name.toLowerCase().includes(search.toLowerCase()),
      );
    }
    return data;
  }, [edges, search.length, search.toLowerCase]);

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
        enableSorting: true,
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
        enableSorting: false,
        cell: (info) => (
          <TableCell>
            <span>{info.getValue()}</span>
          </TableCell>
        ),
      }),
      columnHelper.accessor('port', {
        size: 175,
        minSize: 175,
        header: m.edges_col_last_modified(),
        enableSorting: false,
        cell: () => (
          <TableCell>
            <span>TODO</span>
          </TableCell>
        ),
      }),
      columnHelper.accessor('port', {
        size: 175,
        minSize: 175,
        header: m.edges_col_modified_by(),
        enableSorting: false,
        cell: () => (
          <TableCell>
            <span>TODO</span>
          </TableCell>
        ),
      }),
      columnHelper.accessor('port', {
        size: 175,
        minSize: 175,
        header: m.edges_col_status(),
        enableSorting: false,
        cell: () => (
          <TableCell>
            <span>TODO</span>
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

  // const expandedHeader = useMemo(
  //   () => [
  //     m.users_col_assigned(),
  //     '',
  //     m.users_col_ip(),
  //     m.users_col_connected_through(),
  //     m.users_col_connected_date(),
  //     '',
  //     '',
  //   ],
  //   [],
  // );

  // const renderExpanded = useCallback(
  //   (row: Row<RowData>, isLast = false) =>
  //     row.original.devices.map((device) => {
  //       const latestNetwork = orderBy(
  //         device.networks.filter((n) => isPresent(n.last_connected_at)),
  //         (d) => d.last_connected_at,
  //         ['desc'],
  //       )[0];
  //       const neverConnected = m.profile_devices_col_never_connected();
  //       const ip = latestNetwork?.last_connected_ip ?? neverConnected;
  //       const locationName = latestNetwork?.last_connected_at
  //         ? latestNetwork.network_name
  //         : neverConnected;
  //       const connectionDate = latestNetwork?.last_connected_at
  //         ? displayDate(latestNetwork.last_connected_at)
  //         : neverConnected;
  //       return (
  //         <TableRowContainer
  //           className={clsx({ last: isLast })}
  //           key={device.id}
  //           assignColumnSizing
  //         >
  //           <TableCell empty />
  //           <TableCell alignContent="center" noPadding>
  //             <Icon icon="enter" />
  //           </TableCell>
  //           <TableCell className="device-name-cell">
  //             <Icon icon="devices" />
  //             <span>{device.name}</span>
  //           </TableCell>
  //           <TableCell empty />
  //           <TableCell>
  //             <span>{ip}</span>
  //           </TableCell>
  //           <TableCell>
  //             <span>{locationName}</span>
  //           </TableCell>
  //           <TableCell>
  //             <span>{connectionDate}</span>
  //           </TableCell>
  //           <TableCell empty />
  //           <TableCell empty />
  //           <TableFlexCell />
  //         </TableRowContainer>
  //       );
  //     }),
  //   [],
  // );

  const table = useReactTable({
    initialState: {
      sorting: [
        {
          id: 'name',
          desc: false,
        },
      ],
    },
    state: {
      rowSelection: selected,
      columnFilters: columnFilters,
    },
    columns,
    data: transformedData,
    enableRowSelection: true,
    enableExpanding: true,
    columnResizeMode: 'onChange',
    onColumnFiltersChange: setColumnFilters,
    getFilteredRowModel: getFilteredRowModel(),
    onRowSelectionChange: setSelected,
    getSortedRowModel: getSortedRowModel(),
    getCoreRowModel: getCoreRowModel(),
    getExpandedRowModel: getExpandedRowModel(),
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
