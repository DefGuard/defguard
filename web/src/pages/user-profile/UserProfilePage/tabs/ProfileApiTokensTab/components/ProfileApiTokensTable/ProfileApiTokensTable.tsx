import { useMutation } from '@tanstack/react-query';
import {
  createColumnHelper,
  getCoreRowModel,
  getSortedRowModel,
  useReactTable,
} from '@tanstack/react-table';
import { useMemo } from 'react';
import { m } from '../../../../../../../paraglide/messages';
import api from '../../../../../../../shared/api/api';
import type { ApiToken } from '../../../../../../../shared/api/types';
import { IconButtonMenu } from '../../../../../../../shared/defguard-ui/components/IconButtonMenu/IconButtonMenu';
import { tableEditColumnSize } from '../../../../../../../shared/defguard-ui/components/table/consts';
import { TableBody } from '../../../../../../../shared/defguard-ui/components/table/TableBody/TableBody';
import { TableCell } from '../../../../../../../shared/defguard-ui/components/table/TableCell/TableCell';
import { openModal } from '../../../../../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../../../../../shared/hooks/modalControls/modalTypes';
import { tableSortingFns } from '../../../../../../../shared/utils/dateSortingFn';
import { displayDate } from '../../../../../../../shared/utils/displayDate';
import { useUserProfile } from '../../../../hooks/useUserProfilePage';

type RowData = ApiToken;

const columnHelper = createColumnHelper<RowData>();

export const ProfileApiTokensTable = () => {
  const username = useUserProfile((s) => s.user.username);
  const data = useUserProfile((s) => s.apiTokens);

  const { mutate: deleteApiToken } = useMutation({
    mutationFn: api.user.deleteApiToken,
    meta: {
      invalidate: [['user-overview'], ['user', username, 'api_token']],
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
        enableSorting: true,
        // @ts-expect-error
        sortingFn: 'dateIso',
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
                        testId: 'edit',
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
                        testId: 'delete',
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
    initialState: {
      sorting: [{ id: 'name', desc: false }],
    },
    sortingFns: tableSortingFns,
    columns,
    data: data,
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
    enableRowSelection: false,
    columnResizeMode: 'onChange',
  });

  return <TableBody table={table} />;
};
