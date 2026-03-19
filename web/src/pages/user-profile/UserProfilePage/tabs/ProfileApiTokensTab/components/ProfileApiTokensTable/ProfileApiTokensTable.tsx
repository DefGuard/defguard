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
import { tableEditColumnSize } from '../../../../../../../shared/defguard-ui/components/table/consts';
import { TableBody } from '../../../../../../../shared/defguard-ui/components/table/TableBody/TableBody';
import { TableCell } from '../../../../../../../shared/defguard-ui/components/table/TableCell/TableCell';
import { TableEditCell } from '../../../../../../../shared/defguard-ui/components/table/TableEditCell/TableEditCell';
import { Snackbar } from '../../../../../../../shared/defguard-ui/providers/snackbar/snackbar';
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

  const columns = useMemo(
    () => [
      columnHelper.accessor('name', {
        enableSorting: true,
        header: m.profile_api_col_name(),
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
      columnHelper.accessor('created_at', {
        header: m.col_created_at(),
        size: 175,
        minSize: 175,
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
            <TableEditCell
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
                        openModal(ModalName.ConfirmAction, {
                          title: m.modal_delete_api_token_title(),
                          contentMd: m.modal_delete_api_token_content({
                            name: rowData.name,
                          }),
                          actionPromise: () =>
                            api.user.deleteApiToken({ id: rowData.id, username }),
                          invalidateKeys: [
                            ['user-overview'],
                            ['user', username, 'api_token'],
                          ],
                          submitProps: { text: m.controls_delete(), variant: 'critical' },
                          onSuccess: () =>
                            Snackbar.default(m.modal_delete_api_token_success()),
                          onError: () => Snackbar.error(m.modal_delete_api_token_error()),
                        });
                      },
                    },
                  ],
                },
              ]}
            />
          );
        },
      }),
    ],
    [username],
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
