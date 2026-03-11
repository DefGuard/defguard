import { useMutation, useSuspenseQuery } from '@tanstack/react-query';
import {
  createColumnHelper,
  getCoreRowModel,
  getSortedRowModel,
  useReactTable,
} from '@tanstack/react-table';
import { useMemo } from 'react';
import { m } from '../../paraglide/messages';
import api from '../../shared/api/api';
import type { OpenIdClient } from '../../shared/api/types';
import { Badge } from '../../shared/defguard-ui/components/Badge/Badge';
import { Button } from '../../shared/defguard-ui/components/Button/Button';
import type { ButtonProps } from '../../shared/defguard-ui/components/Button/types';
import { EmptyStateFlexible } from '../../shared/defguard-ui/components/EmptyStateFlexible/EmptyStateFlexible';
import type { MenuItemsGroup } from '../../shared/defguard-ui/components/Menu/types';
import { tableEditColumnSize } from '../../shared/defguard-ui/components/table/consts';
import { TableBody } from '../../shared/defguard-ui/components/table/TableBody/TableBody';
import { TableCell } from '../../shared/defguard-ui/components/table/TableCell/TableCell';
import { TableEditCell } from '../../shared/defguard-ui/components/table/TableEditCell/TableEditCell';
import { TableTop } from '../../shared/defguard-ui/components/table/TableTop/TableTop';
import { useClipboard } from '../../shared/defguard-ui/hooks/useClipboard';
import { openModal } from '../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../shared/hooks/modalControls/modalTypes';
import { getOpenIdClientQueryOptions } from '../../shared/query';

type RowData = OpenIdClient;

const columnHelper = createColumnHelper<RowData>();

export const OpenIdClientTable = () => {
  const { data } = useSuspenseQuery(getOpenIdClientQueryOptions);

  const reservedNames = useMemo(
    () => data.map((c) => c.name.toLowerCase().replaceAll(' ', '')),
    [data],
  );

  const { writeToClipboard } = useClipboard();

  const { mutate: deleteClient } = useMutation({
    mutationFn: api.openIdClient.deleteOpenIdClient,
    meta: {
      invalidate: ['oauth'],
    },
  });

  const { mutate: toggleClient } = useMutation({
    mutationFn: api.openIdClient.changeOpenIdClientState,
    meta: {
      invalidate: ['oauth'],
    },
  });

  const addButtonProps = useMemo(
    (): ButtonProps => ({
      text: 'Add new application',
      iconLeft: 'openid',
      testId: 'add-new-app',
      onClick: () => {
        openModal(ModalName.CEOpenIdClient, {
          reservedNames,
        });
      },
    }),
    [reservedNames],
  );

  const columns = useMemo(
    () => [
      columnHelper.accessor('name', {
        header: 'App name',
        enableSorting: true,
        sortingFn: 'text',
        minSize: 300,
        cell: (info) => (
          <TableCell>
            <span>{info.getValue()}</span>
          </TableCell>
        ),
      }),
      columnHelper.accessor('enabled', {
        header: 'Status',
        minSize: 180,
        cell: (info) => (
          <TableCell>
            {info.getValue() ? (
              <Badge variant="success" text="Enabled" />
            ) : (
              <Badge variant="critical" text="Disabled" />
            )}
          </TableCell>
        ),
      }),
      columnHelper.display({
        id: 'edit',
        header: '',
        size: tableEditColumnSize,
        cell: (info) => {
          const row = info.row.original;
          const menuItems: MenuItemsGroup[] = [
            {
              items: [
                {
                  text: m.controls_edit(),
                  icon: 'edit',
                  onClick: () => {
                    openModal(ModalName.CEOpenIdClient, {
                      reservedNames,
                      openIdClient: row,
                    });
                  },
                },
                {
                  icon: 'activity-notes',
                  text: m.openid_edit_copy_id(),
                  testId: 'copy-id',
                  onClick: () => {
                    writeToClipboard(row.client_id);
                  },
                },
                {
                  icon: 'copy',
                  text: m.openid_edit_copy_secret(),
                  testId: 'copy-secret',
                  onClick: () => {
                    writeToClipboard(row.client_secret);
                  },
                },
                {
                  icon: row.enabled ? 'disabled' : 'check-circle',
                  text: row.enabled ? m.controls_disable() : m.controls_enable(),
                  onClick: () => {
                    toggleClient({
                      client_id: row.client_id,
                      enabled: !row.enabled,
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
                    deleteClient(row.client_id);
                  },
                },
              ],
            },
          ];
          return <TableEditCell menuItems={menuItems} />;
        },
      }),
    ],
    [reservedNames, deleteClient, writeToClipboard, toggleClient],
  );

  const table = useReactTable({
    columns,
    data,
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
    enableRowSelection: false,
    columnResizeMode: 'onChange',
  });

  return (
    <>
      {data.length > 0 && (
        <>
          <TableTop text={m.openid_table_top_title()}>
            <Button {...addButtonProps} />
          </TableTop>
          <TableBody table={table} />
        </>
      )}
      {data.length === 0 && (
        <EmptyStateFlexible
          icon="openid"
          title={m.openid_empty_title()}
          subtitle={m.openid_empty_subtitle()}
          primaryAction={addButtonProps}
        />
      )}
    </>
  );
};
