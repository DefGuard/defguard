import { useMutation } from '@tanstack/react-query';
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
import { IconButtonMenu } from '../../shared/defguard-ui/components/IconButtonMenu/IconButtonMenu';
import type { MenuItemsGroup } from '../../shared/defguard-ui/components/Menu/types';
import { tableEditColumnSize } from '../../shared/defguard-ui/components/table/consts';
import { TableBody } from '../../shared/defguard-ui/components/table/TableBody/TableBody';
import { TableCell } from '../../shared/defguard-ui/components/table/TableCell/TableCell';
import { TableTop } from '../../shared/defguard-ui/components/table/TableTop/TableTop';
import { useClipboard } from '../../shared/defguard-ui/hooks/useClipboard';
import { openModal } from '../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../shared/hooks/modalControls/modalTypes';

type RowData = OpenIdClient;

const columnHelper = createColumnHelper<RowData>();

type Props = {
  data: OpenIdClient[];
};

export const OpenIdClientTable = ({ data }: Props) => {
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
        meta: {
          flex: true,
        },
        cell: (info) => (
          <TableCell>
            <span>{info.getValue()}</span>
          </TableCell>
        ),
      }),
      columnHelper.accessor('enabled', {
        header: 'Status',
        size: 600,
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
                  onClick: () => {
                    writeToClipboard(row.client_id);
                  },
                },
                {
                  icon: 'copy',
                  text: m.openid_edit_copy_secret(),
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
          return (
            <TableCell>
              <IconButtonMenu menuItems={menuItems} icon="menu" />
            </TableCell>
          );
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
          title={m.openid_empty_title()}
          subtitle={m.openid_empty_subtitle()}
          primaryAction={addButtonProps}
        />
      )}
    </>
  );
};
