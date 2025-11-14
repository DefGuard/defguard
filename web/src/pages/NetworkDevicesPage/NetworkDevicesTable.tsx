import {
  createColumnHelper,
  getCoreRowModel,
  getSortedRowModel,
  useReactTable,
} from '@tanstack/react-table';
import { useMemo } from 'react';
import type { NetworkDevice } from '../../shared/api/types';
import { Badge } from '../../shared/defguard-ui/components/Badge/Badge';
import { Button } from '../../shared/defguard-ui/components/Button/Button';
import type { ButtonProps } from '../../shared/defguard-ui/components/Button/types';
import { EmptyStateFlexible } from '../../shared/defguard-ui/components/EmptyStateFlexible/EmptyStateFlexible';
import { SizedBox } from '../../shared/defguard-ui/components/SizedBox/SizedBox';
import { TableBody } from '../../shared/defguard-ui/components/table/TableBody/TableBody';
import { TableCell } from '../../shared/defguard-ui/components/table/TableCell/TableCell';
import { TableTop } from '../../shared/defguard-ui/components/table/TableTop/TableTop';
import { ThemeSize } from '../../shared/defguard-ui/types';
import { displayDate } from '../../shared/utils/displayDate';

type Props = {
  networkDevices: NetworkDevice[];
};

type RowData = NetworkDevice;

const columnHelper = createColumnHelper<RowData>();

export const NetworkDevicesTable = ({ networkDevices }: Props) => {
  const addButtonProps = useMemo(
    (): ButtonProps => ({
      text: 'Add new device',
      iconLeft: 'add-device',
      disabled: true,
    }),
    [],
  );

  const columns = useMemo(
    () => [
      columnHelper.accessor('name', {
        header: 'Device name',
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
      columnHelper.accessor('location.name', {
        header: 'Location',
        size: 250,
        cell: (info) => (
          <TableCell>
            <span>{info.getValue()}</span>
          </TableCell>
        ),
      }),
      columnHelper.accessor('assigned_ips', {
        header: 'Assigned IPs',
        size: 250,
        cell: (info) => (
          <TableCell>
            <span>{info.getValue().join(', ')}</span>
          </TableCell>
        ),
      }),
      columnHelper.accessor('description', {
        header: 'Description',
        size: 370,
        cell: (info) => (
          <TableCell>
            <span>{info.getValue()}</span>
          </TableCell>
        ),
      }),
      columnHelper.accessor('added_by', {
        header: 'Added by',
        size: 140,
        cell: (info) => (
          <TableCell>
            <span>{info.getValue()}</span>
          </TableCell>
        ),
      }),
      columnHelper.accessor('added_date', {
        header: 'Added date',
        size: 150,
        cell: (info) => (
          <TableCell>
            <span>{displayDate(info.getValue())}</span>
          </TableCell>
        ),
      }),
      columnHelper.accessor('configured', {
        header: 'Configured',
        size: 150,
        cell: (info) => (
          <TableCell>
            <span>
              {info.getValue() ? (
                <Badge text="Ready" />
              ) : (
                <Badge icon="pending" variant="warning" text="Awaiting Setup" />
              )}
            </span>
          </TableCell>
        ),
      }),
    ],
    [],
  );

  const table = useReactTable({
    data: networkDevices,
    columns,
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
    enableSorting: true,
    enableRowSelection: false,
  });

  return (
    <>
      {networkDevices.length === 0 && (
        <EmptyStateFlexible
          title="You don't have any network devices."
          subtitle="To add one, click the button below."
          primaryAction={addButtonProps}
        />
      )}
      {networkDevices.length !== 0 && (
        <>
          <SizedBox height={ThemeSize.Xl3} />
          <TableTop text="All network devices">
            <Button {...addButtonProps} />
          </TableTop>
          <TableBody table={table} />
        </>
      )}
    </>
  );
};
