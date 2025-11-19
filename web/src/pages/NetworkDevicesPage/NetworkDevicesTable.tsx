import { useMutation } from '@tanstack/react-query';
import {
  createColumnHelper,
  getCoreRowModel,
  getSortedRowModel,
  useReactTable,
} from '@tanstack/react-table';
import { orderBy } from 'lodash-es';
import { useMemo } from 'react';
import { m } from '../../paraglide/messages';
import api from '../../shared/api/api';
import { LocationMfaMode, type NetworkDevice } from '../../shared/api/types';
import { Badge } from '../../shared/defguard-ui/components/Badge/Badge';
import { Button } from '../../shared/defguard-ui/components/Button/Button';
import type { ButtonProps } from '../../shared/defguard-ui/components/Button/types';
import { EmptyStateFlexible } from '../../shared/defguard-ui/components/EmptyStateFlexible/EmptyStateFlexible';
import { IconButtonMenu } from '../../shared/defguard-ui/components/IconButtonMenu/IconButtonMenu';
import type {
  MenuItemProps,
  MenuItemsGroup,
} from '../../shared/defguard-ui/components/Menu/types';
import { SizedBox } from '../../shared/defguard-ui/components/SizedBox/SizedBox';
import { tableEditColumnSize } from '../../shared/defguard-ui/components/table/consts';
import { TableBody } from '../../shared/defguard-ui/components/table/TableBody/TableBody';
import { TableCell } from '../../shared/defguard-ui/components/table/TableCell/TableCell';
import { TableTop } from '../../shared/defguard-ui/components/table/TableTop/TableTop';
import { ThemeSize } from '../../shared/defguard-ui/types';
import { openModal } from '../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../shared/hooks/modalControls/modalTypes';
import { displayDate } from '../../shared/utils/displayDate';

type Props = {
  networkDevices: NetworkDevice[];
};

type RowData = NetworkDevice;

const columnHelper = createColumnHelper<RowData>();

export const NetworkDevicesTable = ({ networkDevices }: Props) => {
  const reservedNames = useMemo(
    () => networkDevices.map((n) => n.name),
    [networkDevices],
  );

  const { mutate: deleteDevice } = useMutation({
    mutationFn: api.network_device.deleteDevice,
    meta: {
      invalidate: ['device', 'network'],
    },
  });

  const { mutate: openAdd, isPending: addPending } = useMutation({
    mutationFn: async () => {
      const { data: locations } = await api.location.getLocations();
      const availableLocations = locations.filter(
        (location) => location.location_mfa_mode === LocationMfaMode.Disabled,
      );
      if (!availableLocations.length) return;
      const { data: availableIps } = await api.network_device.getAvailableIp(
        availableLocations[0].id,
      );
      openModal(ModalName.AddNetworkDevice, {
        availableIps,
        locations: orderBy(availableLocations, ['name'], ['asc']),
        reservedNames,
      });
    },
  });

  const addButtonProps = useMemo(
    (): ButtonProps => ({
      text: 'Add new device',
      iconLeft: 'add-device',
      loading: addPending,
      onClick: () => {
        openAdd();
      },
    }),
    [addPending, openAdd],
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
      columnHelper.display({
        id: 'edit',
        header: '',
        size: tableEditColumnSize,
        cell: (info) => {
          const row = info.row.original;
          const mainItems: MenuItemProps[] = [
            {
              text: m.controls_edit(),
              icon: 'edit',
              onClick: () => {
                openModal(ModalName.EditNetworkDevice, {
                  device: row,
                  reservedNames: reservedNames,
                });
              },
            },
            {
              text: 'Generate auth token',
              icon: 'token',
              onClick: async () => {
                const { data: enrollment } = await api.network_device.startCliForDevice(
                  row.id,
                );
                openModal(ModalName.NetworkDeviceToken, {
                  device: row,
                  enrollment,
                });
              },
            },
          ];
          if (row.configured) {
            mainItems.splice(1, 0, {
              text: 'View config',
              icon: 'config',
              onClick: async () => {
                const { data: config } = await api.network_device.getDeviceConfig(row.id);
                openModal(ModalName.NetworkDeviceConfig, {
                  config,
                  device: row,
                });
              },
            });
          }
          const menuItems: MenuItemsGroup[] = [
            {
              items: mainItems,
            },
            {
              items: [
                {
                  text: m.controls_delete(),
                  icon: 'delete',
                  onClick: () => {
                    deleteDevice(row.id);
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
    [reservedNames, deleteDevice],
  );

  const table = useReactTable({
    initialState: {
      sorting: [{ id: 'name', desc: false }],
    },
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
