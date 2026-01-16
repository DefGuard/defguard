import { useMutation } from '@tanstack/react-query';
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
import { IconButtonMenu } from '../../../shared/defguard-ui/components/IconButtonMenu/IconButtonMenu';
import type { MenuItemsGroup } from '../../../shared/defguard-ui/components/Menu/types';
import { tableEditColumnSize } from '../../../shared/defguard-ui/components/table/consts';
import { TableBody } from '../../../shared/defguard-ui/components/table/TableBody/TableBody';
import { TableCell } from '../../../shared/defguard-ui/components/table/TableCell/TableCell';
import { openModal } from '../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../shared/hooks/modalControls/modalTypes';

type RowData = ActivityLogStream;

const columnHelper = createColumnHelper<RowData>();

type Props = {
  data: RowData[];
};

export const ActivityLogStreamTable = ({ data: rowData }: Props) => {
  const { mutate: deleteStream } = useMutation({
    mutationFn: api.activityLogStream.deleteStream, //TODO
    meta: {
      invalidate: ['activity_log_stream'],
    },
  });

  const columns = useMemo(
    () => [
      columnHelper.accessor('name', {
        header: 'Name',
        minSize: 484,
        enableSorting: true,
        sortingFn: 'text',
        cell: (info) => (
          <TableCell>
            <span>{info.getValue()}</span>
          </TableCell>
        ),
      }),
      columnHelper.accessor('stream_type', {
        header: 'Destination',
        size: 200,
        minSize: 100,
        cell: (info) => {
          const value = info.getValue();
          const displayValue = value === 'vector_http' ? 'Vector' : 'Logstash';
          return (
            <TableCell>
              <span>{displayValue}</span>
            </TableCell>
          );
        },
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
                    // TODO: edit modal
                    openModal(ModalName.AddLogStreaming);
                  },
                },
                {
                  text: m.controls_delete(),
                  icon: 'delete',
                  variant: 'danger',
                  onClick: () => {
                    deleteStream(row.id);
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
    [deleteStream],
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
