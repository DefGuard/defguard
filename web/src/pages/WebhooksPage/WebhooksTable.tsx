import { useMutation } from '@tanstack/react-query';
import {
  createColumnHelper,
  getCoreRowModel,
  useReactTable,
} from '@tanstack/react-table';
import { useMemo } from 'react';
import { m } from '../../paraglide/messages';
import api from '../../shared/api/api';
import type { Webhook } from '../../shared/api/types';
import { Badge } from '../../shared/defguard-ui/components/Badge/Badge';
import { Button } from '../../shared/defguard-ui/components/Button/Button';
import type { ButtonProps } from '../../shared/defguard-ui/components/Button/types';
import { EmptyStateFlexible } from '../../shared/defguard-ui/components/EmptyStateFlexible/EmptyStateFlexible';
import { IconButtonMenu } from '../../shared/defguard-ui/components/IconButtonMenu/IconButtonMenu';
import type { MenuItemsGroup } from '../../shared/defguard-ui/components/Menu/types';
import { TableBody } from '../../shared/defguard-ui/components/table/TableBody/TableBody';
import { TableCell } from '../../shared/defguard-ui/components/table/TableCell/TableCell';
import { TableTop } from '../../shared/defguard-ui/components/table/TableTop/TableTop';
import { openModal } from '../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../shared/hooks/modalControls/modalTypes';

type RowData = Webhook;

const columnHelper = createColumnHelper<RowData>();

type Props = {
  webhooks: Webhook[];
};

export const WebhooksTable = ({ webhooks }: Props) => {
  const { mutate: toggleWebhook } = useMutation({
    mutationFn: api.webhook.changeWebhookState,
    meta: {
      invalidate: ['webhook'],
    },
  });
  const { mutate: deleteWebhook } = useMutation({
    mutationFn: api.webhook.deleteWebhook,
    meta: {
      invalidate: ['webhook'],
    },
  });
  const addButtonProps = useMemo(
    (): ButtonProps => ({
      text: m.webhooks_add(),
      iconLeft: 'webhooks',
      onClick: () => {
        openModal(ModalName.CEWebhook, {});
      },
    }),
    [],
  );

  const columns = useMemo(
    () => [
      columnHelper.accessor('url', {
        header: 'Webhook URL',
        meta: {
          flex: true,
        },
        cell: (info) => (
          <TableCell>
            <span>{info.getValue()}</span>
          </TableCell>
        ),
      }),
      columnHelper.accessor('description', {
        header: 'Description',
        size: 625,
        cell: (info) => (
          <TableCell>
            <span>{info.getValue()}</span>
          </TableCell>
        ),
      }),
      columnHelper.accessor('enabled', {
        header: 'Status',
        size: 300,
        cell: (info) => (
          <TableCell>
            {info.getValue() ? (
              <Badge variant="success" text={m.misc_active()} />
            ) : (
              <Badge variant="critical" text={m.state_disabled()} />
            )}
          </TableCell>
        ),
      }),
      columnHelper.display({
        id: 'edit',
        header: '',
        cell: (info) => {
          const row = info.row.original;
          const menuItems: MenuItemsGroup[] = [
            {
              items: [
                {
                  text: m.controls_edit(),
                  icon: 'edit',
                  onClick: () => {
                    openModal(ModalName.CEWebhook, {
                      webhook: row,
                    });
                  },
                },
                {
                  text: row.enabled ? m.controls_disable() : m.controls_enable(),
                  icon: row.enabled ? 'disabled' : 'check-circle',
                  onClick: () => {
                    toggleWebhook({
                      enabled: !row.enabled,
                      id: row.id,
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
                    deleteWebhook(row.id);
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
    [deleteWebhook, toggleWebhook],
  );
  const table = useReactTable({
    columns,
    data: webhooks,
    getCoreRowModel: getCoreRowModel(),
    enableRowSelection: false,
  });
  return (
    <>
      {webhooks.length > 0 && (
        <>
          <TableTop text="All Webhooks">
            <Button {...addButtonProps} />
          </TableTop>
          <TableBody table={table} />
        </>
      )}
      {webhooks.length === 0 && (
        <EmptyStateFlexible
          icon="webhook"
          title={m.webhooks_empty_title()}
          subtitle={m.webhooks_empty_subtitle()}
          primaryAction={addButtonProps}
        />
      )}
    </>
  );
};
