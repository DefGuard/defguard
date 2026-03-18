import {
  createColumnHelper,
  getCoreRowModel,
  getSortedRowModel,
  useReactTable,
} from '@tanstack/react-table';
import { useMemo } from 'react';
import { m } from '../../../paraglide/messages';
import api from '../../../shared/api/api';
import type { ActivityLogStream } from '../../../shared/api/types';
import type { MenuItemsGroup } from '../../../shared/defguard-ui/components/Menu/types';
import { tableEditColumnSize } from '../../../shared/defguard-ui/components/table/consts';
import { TableBody } from '../../../shared/defguard-ui/components/table/TableBody/TableBody';
import { TableCell } from '../../../shared/defguard-ui/components/table/TableCell/TableCell';
import { TableEditCell } from '../../../shared/defguard-ui/components/table/TableEditCell/TableEditCell';
import { Snackbar } from '../../../shared/defguard-ui/providers/snackbar/snackbar';
import { openModal } from '../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../shared/hooks/modalControls/modalTypes';

type RowData = ActivityLogStream;

const columnHelper = createColumnHelper<RowData>();

type Props = {
  data: RowData[];
};

export const ActivityLogStreamTable = ({ data: rowData }: Props) => {
  const columns = useMemo(
    () => [
      columnHelper.accessor('name', {
        header: m.settings_activity_log_streaming_table_header_name(),
        enableSorting: true,
        sortingFn: 'text',
        size: 300,
        minSize: 200,
        meta: {
          flex: true,
        },
        cell: (info) => (
          <TableCell>
            <span>{info.getValue()}</span>
          </TableCell>
        ),
      }),
      columnHelper.accessor('stream_type', {
        header: m.settings_activity_log_streaming_table_stream_type_name(),
        size: 200,
        minSize: 100,
        cell: (info) => (
          <TableCell>
            <span>
              {info.getValue() === 'vector_http'
                ? m.modal_vector_destination_title()
                : m.modal_logstash_destination_title()}
            </span>
          </TableCell>
        ),
      }),
      columnHelper.display({
        id: 'edit',
        enableResizing: false,
        header: '',
        enableSorting: false,
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
                    openModal(ModalName.EditLogStreaming, row);
                  },
                },
                {
                  text: m.controls_delete(),
                  icon: 'delete',
                  variant: 'danger',
                  onClick: () => {
                    openModal(ModalName.ConfirmAction, {
                      title:
                        m.settings_activity_log_streaming_delete_log_streaming_title(),
                      contentMd: m.modal_delete_logstream_destination(),
                      actionPromise: () => api.activityLogStream.deleteStream(row.id),
                      invalidateKeys: [['activity_log_stream']],
                      submitProps: { text: m.controls_delete(), variant: 'critical' },
                      onError: () =>
                        Snackbar.error(
                          m.settings_activity_log_streaming_delete_log_streaming_failed(),
                        ),
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
    [],
  );

  const table = useReactTable({
    columns,
    data: rowData,
    enableRowSelection: false,
    columnResizeMode: 'onChange',
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
  });

  return <TableBody table={table} />;
};
