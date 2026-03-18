import { useMutation } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import {
  createColumnHelper,
  getCoreRowModel,
  getSortedRowModel,
  useReactTable,
} from '@tanstack/react-table';
import type { AxiosError } from 'axios';
import { orderBy } from 'lodash-es';
import { useMemo } from 'react';
import { m } from '../../paraglide/messages';
import api from '../../shared/api/api';
import { getApiErrorMessage } from '../../shared/api/apiErrorMessages';
import {
  type ApiError,
  LocationMfaMode,
  type NetworkDevice,
} from '../../shared/api/types';
import { Badge } from '../../shared/defguard-ui/components/Badge/Badge';
import { Button } from '../../shared/defguard-ui/components/Button/Button';
import type { ButtonProps } from '../../shared/defguard-ui/components/Button/types';
import { EmptyStateFlexible } from '../../shared/defguard-ui/components/EmptyStateFlexible/EmptyStateFlexible';
import type {
  MenuItemProps,
  MenuItemsGroup,
} from '../../shared/defguard-ui/components/Menu/types';
import { SizedBox } from '../../shared/defguard-ui/components/SizedBox/SizedBox';
import { tableEditColumnSize } from '../../shared/defguard-ui/components/table/consts';
import { TableBody } from '../../shared/defguard-ui/components/table/TableBody/TableBody';
import { TableCell } from '../../shared/defguard-ui/components/table/TableCell/TableCell';
import { TableEditCell } from '../../shared/defguard-ui/components/table/TableEditCell/TableEditCell';
import { TableTop } from '../../shared/defguard-ui/components/table/TableTop/TableTop';
import { Snackbar } from '../../shared/defguard-ui/providers/snackbar/snackbar';
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
  const navigate = useNavigate();
  const reservedNames = useMemo(
    () => networkDevices.map((n) => n.name),
    [networkDevices],
  );

  const { mutate: openAdd, isPending: addPending } = useMutation({
    mutationFn: async () => {
      const { data: locations } = await api.location.getLocations();
      const availableLocations = orderBy(
        locations.filter(
          (location) => location.location_mfa_mode === LocationMfaMode.Disabled,
        ),
        ['name'],
        ['asc'],
      );
      if (!availableLocations.length) {
        openModal(ModalName.ConfirmAction, {
          title: m.modal_no_available_locations_title(),
          contentMd: m.modal_no_available_locations_body(),
          actionPromise: async () => navigate({ to: '/locations' }),
          submitProps: { text: m.modal_no_available_locations_go_to_locations() },
        });
        return;
      }
      const { data: availableIps } = await api.network_device.getAvailableIp(
        availableLocations[0].id,
      );
      openModal(ModalName.AddNetworkDevice, {
        availableIps,
        locations: availableLocations,
        reservedNames,
      });
    },
    onError: (e) => {
      console.error(e);
      const code = (e as AxiosError<ApiError>).response?.data?.code;
      if (code) {
        Snackbar.error(getApiErrorMessage(code));
      } else {
        Snackbar.error(m.network_device_add_error());
      }
    },
  });

  const addButtonProps = useMemo(
    (): ButtonProps => ({
      text: 'Add new device',
      iconLeft: 'add-device',
      loading: addPending,
      testId: 'add-device',
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
        minSize: 250,
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
        size: 225,
        minSize: 175,
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
        size: 300,
        minSize: 250,
        cell: (info) => (
          <TableCell>
            <span>{info.getValue()}</span>
          </TableCell>
        ),
      }),
      columnHelper.accessor('added_by', {
        header: 'Added by',
        size: 140,
        minSize: 100,
        cell: (info) => (
          <TableCell>
            <span>{info.getValue()}</span>
          </TableCell>
        ),
      }),
      columnHelper.accessor('added_date', {
        header: 'Added date',
        size: 170,
        minSize: 170,
        cell: (info) => (
          <TableCell>
            <span>{displayDate(info.getValue())}</span>
          </TableCell>
        ),
      }),
      columnHelper.accessor('configured', {
        header: 'Configured',
        size: 150,
        minSize: 125,
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
        enableResizing: false,
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
              testId: 'generate-auth-token',
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
                    openModal(ModalName.DeleteNetworkDevice, {
                      id: row.id,
                      name: row.name,
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
    [reservedNames],
  );

  const table = useReactTable({
    initialState: {
      sorting: [{ id: 'name', desc: false }],
    },
    data: networkDevices,
    columns,
    columnResizeMode: 'onChange',
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
    enableSorting: true,
    enableRowSelection: false,
  });

  return (
    <>
      {networkDevices.length === 0 && (
        <EmptyStateFlexible
          icon="devices"
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
