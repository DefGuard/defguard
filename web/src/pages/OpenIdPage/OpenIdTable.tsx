import { useMutation } from '@tanstack/react-query';
import {
  createColumnHelper,
  getCoreRowModel,
  type SortingState,
  useReactTable,
} from '@tanstack/react-table';
import { orderBy } from 'lodash-es';
import { useMemo, useState } from 'react';
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

type SortKey = 'name';

export const OpenIdClientTable = ({ data }: Props) => {
  const [sortingState, setSortingState] = useState<SortingState>([
    {
      desc: false,
      id: 'name',
    },
  ]);

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

  const transformedData = useMemo(() => {
    const result = data;
    const sorting = sortingState[0];
    if (sorting) {
      const key = sorting.id as SortKey;
      const direction = sorting.desc ? 'desc' : 'asc';
      return orderBy(result, (c) => c[key].toLowerCase().replaceAll(' ', ''), [
        direction,
      ]);
    }
    return result;
  }, [data, sortingState[0]]);

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
                  icon: 'copy',
                  text: 'Copy client ID',
                  onClick: () => {
                    writeToClipboard(row.client_id);
                  },
                },
                {
                  icon: 'copy',
                  text: 'Copy client secret',
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
    state: {
      sorting: sortingState,
    },
    columns,
    data: transformedData,
    getCoreRowModel: getCoreRowModel(),
    manualSorting: true,
    onSortingChange: setSortingState,
  });

  return (
    <>
      <TableTop text="All apps">
        <Button {...addButtonProps} />
      </TableTop>
      {transformedData.length > 0 && <TableBody table={table} />}
      {data.length === 0 && (
        <EmptyStateFlexible
          title="You don't have any OpenID Apps."
          subtitle="To add one, click the button below."
          primaryAction={addButtonProps}
        />
      )}
    </>
  );
};
