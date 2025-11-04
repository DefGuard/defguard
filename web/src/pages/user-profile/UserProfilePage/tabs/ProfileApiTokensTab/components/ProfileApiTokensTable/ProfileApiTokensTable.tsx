import { useMutation } from '@tanstack/react-query';
import {
  createColumnHelper,
  getCoreRowModel,
  type SortingState,
  useReactTable,
} from '@tanstack/react-table';
import { orderBy } from 'lodash-es';
import { useMemo, useState } from 'react';
import { m } from '../../../../../../../paraglide/messages';
import api from '../../../../../../../shared/api/api';
import type { ApiToken } from '../../../../../../../shared/api/types';
import { IconButtonMenu } from '../../../../../../../shared/defguard-ui/components/IconButtonMenu/IconButtonMenu';
import { tableEditColumnSize } from '../../../../../../../shared/defguard-ui/components/table/consts';
import { TableBody } from '../../../../../../../shared/defguard-ui/components/table/TableBody/TableBody';
import { TableCell } from '../../../../../../../shared/defguard-ui/components/table/TableCell/TableCell';
import { openModal } from '../../../../../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../../../../../shared/hooks/modalControls/modalTypes';
import { displayDate } from '../../../../../../../shared/utils/displayDate';
import { useUserProfile } from '../../../../hooks/useUserProfilePage';

type RowData = ApiToken;

const columnHelper = createColumnHelper<RowData>();

type SortingKey = 'name' | 'created_at';

export const ProfileApiTokensTable = () => {
  const username = useUserProfile((s) => s.user.username);
  const data = useUserProfile((s) => s.apiTokens);

  const [sortingState, setSortingState] = useState<SortingState>([
    {
      id: 'name',
      desc: false,
    },
  ]);

  const transformedData = useMemo(() => {
    const sorting = sortingState[0];
    if (!sorting) return data;
    const sortingId = sorting.id as SortingKey;
    const direction = sorting.desc ? 'desc' : 'asc';
    if (sortingId === 'name') {
      return orderBy(data, (o) => o.name.trim().toLowerCase().replaceAll(' ', ''), [
        direction,
      ]);
    }
    // created at
    return orderBy(data, (o) => o.created_at, [direction]);
  }, [data, sortingState[0]]);

  const { mutate: deleteApiToken } = useMutation({
    mutationFn: api.user.deleteApiToken,
    meta: {
      invalidate: ['user', username, 'api_token'],
    },
  });

  const columns = useMemo(
    () => [
      columnHelper.accessor('name', {
        enableSorting: true,
        header: m.profile_api_col_name(),
        cell: (info) => (
          <TableCell>
            <span>{info.getValue()}</span>
          </TableCell>
        ),
        meta: {
          flex: true,
        },
      }),
      columnHelper.accessor('created_at', {
        header: m.col_created_at(),
        size: 175,
        cell: (info) => (
          <TableCell>
            <span>{displayDate(info.getValue())}</span>
          </TableCell>
        ),
      }),
      columnHelper.display({
        id: 'edit',
        header: '',
        size: tableEditColumnSize,
        cell: (info) => {
          const rowData = info.row.original;

          return (
            <TableCell>
              <IconButtonMenu
                icon="menu"
                menuItems={[
                  {
                    items: [
                      {
                        icon: 'edit',
                        text: m.controls_rename(),
                        onClick: () => {
                          openModal(ModalName.RenameApiToken, {
                            id: rowData.id,
                            name: rowData.name,
                            username,
                          });
                        },
                      },
                      {
                        icon: 'delete',
                        variant: 'danger',
                        text: m.controls_delete(),
                        onClick: () => {
                          deleteApiToken({
                            id: rowData.id,
                            username,
                          });
                        },
                      },
                    ],
                  },
                ]}
              />
            </TableCell>
          );
        },
      }),
    ],
    [deleteApiToken, username],
  );

  const table = useReactTable({
    state: {
      sorting: sortingState,
    },
    columns,
    data: transformedData,
    manualSorting: true,
    getCoreRowModel: getCoreRowModel(),
    onSortingChange: setSortingState,
  });

  return <TableBody table={table} />;
};
