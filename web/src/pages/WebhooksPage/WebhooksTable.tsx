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
import type { MenuItemsGroup } from '../../shared/defguard-ui/components/Menu/types';
import { tableEditColumnSize } from '../../shared/defguard-ui/components/table/consts';
import { TableBody } from '../../shared/defguard-ui/components/table/TableBody/TableBody';
import { TableCell } from '../../shared/defguard-ui/components/table/TableCell/TableCell';
import { TableEditCell } from '../../shared/defguard-ui/components/table/TableEditCell/TableEditCell';
import { TableTop } from '../../shared/defguard-ui/components/table/TableTop/TableTop';
import { Snackbar } from '../../shared/defguard-ui/providers/snackbar/snackbar';
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
  const addButtonProps = useMemo(
    (): ButtonProps => ({
      text: m.webhooks_add(),
      iconLeft: 'webhooks',
      testId: 'add-new-webhook',
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
        minSize: 300,
        size: 450,
        cell: (info) => (
          <TableCell>
            <span>{info.getValue()}</span>
          </TableCell>
        ),
      }),
      columnHelper.accessor('description', {
        header: 'Description',
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
      columnHelper.accessor('enabled', {
        header: 'Status',
        minSize: 125,
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
        size: tableEditColumnSize,
        cell: (info) => {
          const row = info.row.original;
          const menuItems: MenuItemsGroup[] = [
            {
              items: [
                {
                  text: m.controls_edit(),
                  icon: 'edit',
                  testId: 'edit',
                  onClick: () => {
                    openModal(ModalName.CEWebhook, {
                      webhook: row,
                    });
                  },
                },
                {
                  text: row.enabled ? m.controls_disable() : m.controls_enable(),
                  icon: row.enabled ? 'disabled' : 'check-circle',
                  testId: 'change-state',
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
                  testId: 'delete',
                  variant: 'danger',
                  onClick: () => {
                    openModal(ModalName.ConfirmAction, {
                      title: m.webhooks_delete_confirm_title(),
                      contentMd: m.webhooks_delete_confirm_body(),
                      actionPromise: () => api.webhook.deleteWebhook(row.id),
                      invalidateKeys: [['webhook']],
                      submitProps: { text: m.controls_delete(), variant: 'critical' },
                      onSuccess: () => Snackbar.default(m.webhooks_delete_success()),
                      onError: () => Snackbar.error(m.webhooks_delete_failed()),
                    });
                  },
                },
              ],
            },
          ];
          return <TableEditCell menuItems={menuItems} />;
        },
      }),
    ],
    [toggleWebhook],
  );
  const table = useReactTable({
    columns,
    data: webhooks,
    getCoreRowModel: getCoreRowModel(),
    enableRowSelection: false,
    columnResizeMode: 'onChange',
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
