import { keepPreviousData, useInfiniteQuery, useQuery } from '@tanstack/react-query';
import {
  type ColumnFiltersState,
  createColumnHelper,
  getCoreRowModel,
  type RowSelectionState,
  type SortingState,
  useReactTable,
} from '@tanstack/react-table';
import { useMemo, useState } from 'react';
import { m } from '../../../../paraglide/messages';
import api from '../../../../shared/api/api';
import {
  DeviceAttestationStatus,
  type DeviceAttestationStatusValue,
  type DeviceOverview,
  type DeviceOverviewListParams,
  type DeviceOverviewSortKey,
  DevicePlatform,
  type DevicePlatformValue,
  DeviceState,
  type DeviceStateValue,
  type DeviceSystemFilterValue,
  DeviceVerification,
  type DeviceVerificationValue,
  deviceSystemUnknown,
  SortDirection,
} from '../../../../shared/api/types';
import type { SelectionOption } from '../../../../shared/components/SelectionSection/type';
import { PolicyOsVariant } from '../../../../shared/components/SystemSelector/types';
import { TableSkeleton } from '../../../../shared/components/skeleton/TableSkeleton/TableSkeleton';
import { AppText } from '../../../../shared/defguard-ui/components/AppText/AppText';
import { Badge } from '../../../../shared/defguard-ui/components/Badge/Badge';
import {
  BadgeVariant,
  type BadgeVariantValue,
} from '../../../../shared/defguard-ui/components/Badge/types';
import { ButtonMenu } from '../../../../shared/defguard-ui/components/ButtonMenu/MenuButton';
import { EmptyStateFlexible } from '../../../../shared/defguard-ui/components/EmptyStateFlexible/EmptyStateFlexible';
import { Icon } from '../../../../shared/defguard-ui/components/Icon';
import { Search } from '../../../../shared/defguard-ui/components/Search/Search';
import { tableEditColumnSize } from '../../../../shared/defguard-ui/components/table/consts';
import { TableBody } from '../../../../shared/defguard-ui/components/table/TableBody/TableBody';
import { TableCell } from '../../../../shared/defguard-ui/components/table/TableCell/TableCell';
import { TableEditCell } from '../../../../shared/defguard-ui/components/table/TableEditCell/TableEditCell';
import { TableTop } from '../../../../shared/defguard-ui/components/table/TableTop/TableTop';
import { TextStyle, ThemeVariable } from '../../../../shared/defguard-ui/types';
import { isPresent } from '../../../../shared/defguard-ui/utils/isPresent';
import { getUsersDisplayQueryOptions } from '../../../../shared/query';
import {
  policyOsVariantToIcon,
  policyOsVariantToText,
} from '../../../../shared/utils/policyPostures';
import {
  attestationStatusText,
  devicePlatformText,
  osIconSize,
  verificationText,
} from '../../consts';
import { platformIcon, systemText } from '../../helpers';
import { useDeviceActions } from '../../hooks/useDeviceActions';
import { useEnterpriseTier } from '../../hooks/useEnterpriseTier';
import { BlockedBadge } from '../BlockedBadge/BlockedBadge';
import { ConnectionBadge } from '../ConnectionBadge/ConnectionBadge';
import { DeviceDetailsDrawer } from '../DeviceDetailsDrawer/DeviceDetailsDrawer';
import { UnknownBadge } from '../UnknownBadge/UnknownBadge';
import { DeviceNameButton } from './components/DeviceNameButton/DeviceNameButton';

const columnHelper = createColumnHelper<DeviceOverview>();

const verificationContent = (verification: DeviceVerificationValue) => {
  switch (verification) {
    case DeviceVerification.Verified:
      return (
        <AppText font={TextStyle.TBodySm400} color={ThemeVariable.FgMuted}>
          {m.devices_verification_verified()}
        </AppText>
      );
    case DeviceVerification.Pending:
      return (
        <Badge
          variant={BadgeVariant.Additional}
          icon="status-pending"
          showIcon
          text={m.devices_verification_pending()}
        />
      );
    case DeviceVerification.Unverified:
      return null;
  }
};

const attestationBadge: Record<
  Exclude<DeviceAttestationStatusValue, typeof DeviceAttestationStatus.NotAttested>,
  { text: () => string; variant: BadgeVariantValue }
> = {
  [DeviceAttestationStatus.Attested]: {
    text: m.devices_attestation_attested,
    variant: BadgeVariant.Success,
  },
  [DeviceAttestationStatus.Unrecognized]: {
    text: m.devices_attestation_unrecognized,
    variant: BadgeVariant.Warning,
  },
  [DeviceAttestationStatus.AttestedManually]: {
    text: m.devices_attestation_attested_manually,
    variant: BadgeVariant.Success,
  },
  [DeviceAttestationStatus.AttestationFailed]: {
    text: m.devices_attestation_failed,
    variant: BadgeVariant.Critical,
  },
};

const ConnectionFilter = {
  Online: 'online',
  Offline: 'offline',
} as const;

type ConnectionFilterValue = (typeof ConnectionFilter)[keyof typeof ConnectionFilter];

const getFilterValues = <T,>(columnFilters: ColumnFiltersState, columnId: string) => {
  const value = columnFilters.find((filter) => filter.id === columnId)?.value;
  return Array.isArray(value) && value.length > 0 ? (value as T[]) : undefined;
};

export const DevicesOverviewTable = () => {
  const [search, setSearch] = useState('');
  const [sorting, setSorting] = useState<SortingState>([{ id: 'name', desc: false }]);
  const [columnFilters, setColumnFilters] = useState<ColumnFiltersState>([]);
  const isEnterprise = useEnterpriseTier();

  const requestParams = useMemo((): DeviceOverviewListParams => {
    const sort = sorting[0];
    const connection = getFilterValues<ConnectionFilterValue>(columnFilters, 'connected');
    return {
      search: search.length ? search : undefined,
      sort_by: sort?.id as DeviceOverviewSortKey | undefined,
      sort_order: sort ? (sort.desc ? SortDirection.DESC : SortDirection.ASC) : undefined,
      state: getFilterValues<DeviceStateValue>(columnFilters, 'state'),
      verification: isEnterprise
        ? getFilterValues<DeviceVerificationValue>(columnFilters, 'verification')
        : undefined,
      attestation_status: isEnterprise
        ? getFilterValues<DeviceAttestationStatusValue>(
            columnFilters,
            'attestation_status',
          )
        : undefined,
      platform: getFilterValues<DevicePlatformValue>(columnFilters, 'platform'),
      system: getFilterValues<DeviceSystemFilterValue>(columnFilters, 'system'),
      connected:
        connection?.length === 1 ? connection[0] === ConnectionFilter.Online : undefined,
    };
  }, [search, sorting, columnFilters, isEnterprise]);

  const { data, fetchNextPage, isFetchingNextPage, isLoading } = useInfiniteQuery({
    queryKey: ['device', 'overview', requestParams],
    initialPageParam: 1,
    queryFn: ({ pageParam }) =>
      api.device.getDevicesOverview({
        ...requestParams,
        page: pageParam,
      }),
    getNextPageParam: (lastPage) => lastPage.pagination.next_page,
    placeholderData: keepPreviousData,
  });

  const { data: usersDisplay } = useQuery(getUsersDisplayQueryOptions);

  const devices = useMemo(() => data?.pages.flatMap((page) => page.data) ?? [], [data]);
  const hasNextPage = isPresent(data?.pages.at(-1)?.pagination.next_page);

  const [selectedDeviceId, setSelectedDeviceId] = useState<number | null>(null);
  const selectedDevice = useMemo(
    () => devices.find((device) => device.id === selectedDeviceId) ?? null,
    [devices, selectedDeviceId],
  );

  const [rowSelection, setRowSelection] = useState<RowSelectionState>({});
  const selectedDevices = useMemo(
    () => devices.filter((device) => rowSelection[String(device.id)]),
    [devices, rowSelection],
  );
  const { getMenuItems, getRowMenuItems } = useDeviceActions();
  const bulkMenuItems = useMemo(
    () =>
      getMenuItems({
        devices: selectedDevices,
        testIdPrefix: 'devices-bulk',
        onSuccess: () => setRowSelection({}),
      }),
    [getMenuItems, selectedDevices],
  );

  const tableFilterMessages = useMemo(
    () => ({
      searchPlaceholder: m.controls_search(),
      clearButton: m.controls_reset(),
      applyButton: m.controls_submit(),
      emptyState: m.search_empty_common_title(),
    }),
    [],
  );

  const filterOptions = useMemo(
    () => ({
      state: [
        { id: DeviceState.Enabled, label: m.state_enabled() },
        { id: DeviceState.Blocked, label: m.devices_state_blocked() },
      ] satisfies SelectionOption<DeviceStateValue>[],
      attestation: Object.values(DeviceAttestationStatus).map(
        (status): SelectionOption<DeviceAttestationStatusValue> => ({
          id: status,
          label: attestationStatusText[status](),
        }),
      ),
      verification: Object.values(DeviceVerification).map(
        (verification): SelectionOption<DeviceVerificationValue> => ({
          id: verification,
          label: verificationText[verification](),
        }),
      ),
      platform: Object.values(DevicePlatform).map(
        (platform): SelectionOption<DevicePlatformValue> => ({
          id: platform,
          label: devicePlatformText[platform](),
        }),
      ),
      system: [
        ...Object.values(PolicyOsVariant).map(
          (osType): SelectionOption<DeviceSystemFilterValue> => ({
            id: osType,
            label: policyOsVariantToText(osType),
          }),
        ),
        { id: deviceSystemUnknown, label: m.devices_unknown() },
      ],
      connection: [
        { id: ConnectionFilter.Online, label: m.devices_connection_online() },
        { id: ConnectionFilter.Offline, label: m.devices_connection_offline() },
      ] satisfies SelectionOption<ConnectionFilterValue>[],
    }),
    [],
  );

  const columns = useMemo(
    () => [
      columnHelper.accessor('name', {
        header: m.form_label_device_name(),
        enableSorting: true,
        minSize: 250,
        meta: {
          flex: true,
        },
        cell: (info) => (
          <TableCell>
            <DeviceNameButton
              text={info.getValue()}
              testId="device-name"
              onClick={() => setSelectedDeviceId(info.row.original.id)}
            />
          </TableCell>
        ),
      }),
      columnHelper.accessor('state', {
        header: m.devices_col_state(),
        enableSorting: false,
        enableColumnFilter: true,
        meta: { filterOptions: filterOptions.state },
        size: 120,
        minSize: 100,
        cell: (info) => {
          const isEnabled = info.getValue() === DeviceState.Enabled;
          return (
            <TableCell>
              {isEnabled && (
                <AppText font={TextStyle.TBodySm400} color={ThemeVariable.FgMuted}>
                  {m.state_enabled()}
                </AppText>
              )}
              {!isEnabled && <BlockedBadge />}
            </TableCell>
          );
        },
      }),
      ...(isEnterprise
        ? [
            columnHelper.accessor('attestation_status', {
              header: m.devices_col_attestation(),
              enableSorting: false,
              enableColumnFilter: true,
              meta: { filterOptions: filterOptions.attestation },
              size: 170,
              minSize: 140,
              cell: (info) => {
                const status = info.getValue();
                if (!isPresent(status) || status === DeviceAttestationStatus.NotAttested)
                  return <TableCell />;
                const { text, variant } = attestationBadge[status];
                return (
                  <TableCell>
                    <Badge variant={variant} text={text()} />
                  </TableCell>
                );
              },
            }),
            columnHelper.accessor('verification', {
              header: m.devices_col_verification(),
              enableSorting: false,
              enableColumnFilter: true,
              meta: { filterOptions: filterOptions.verification },
              size: 150,
              minSize: 120,
              cell: (info) => {
                const verification = info.getValue();
                return (
                  <TableCell>
                    {isPresent(verification) && verificationContent(verification)}
                  </TableCell>
                );
              },
            }),
          ]
        : []),
      columnHelper.accessor('os_type', {
        id: 'platform',
        header: m.devices_col_platform(),
        enableSorting: false,
        enableColumnFilter: true,
        meta: { filterOptions: filterOptions.platform },
        size: 130,
        minSize: 120,
        cell: (info) => {
          const osType = info.getValue();
          return (
            <TableCell>
              {isPresent(osType) && (
                <Icon
                  icon={platformIcon(osType)}
                  size={osIconSize}
                  staticColor={ThemeVariable.FgNeutral}
                />
              )}
              {!isPresent(osType) && <UnknownBadge />}
            </TableCell>
          );
        },
      }),
      columnHelper.accessor('os_type', {
        id: 'system',
        header: m.devices_col_system(),
        enableSorting: false,
        enableColumnFilter: true,
        meta: { filterOptions: filterOptions.system },
        size: 200,
        minSize: 150,
        cell: (info) => {
          const osType = info.getValue();
          if (!isPresent(osType))
            return (
              <TableCell>
                <UnknownBadge />
              </TableCell>
            );
          const osVersion = info.row.original.os_version;
          return (
            <TableCell>
              <Icon
                icon={policyOsVariantToIcon(osType)}
                size={osIconSize}
                staticColor={ThemeVariable.FgNeutral}
              />
              <span>{systemText(osType, osVersion)}</span>
            </TableCell>
          );
        },
      }),
      columnHelper.accessor('client_version', {
        header: m.devices_col_client_version(),
        enableSorting: false,
        size: 140,
        minSize: 120,
        cell: (info) => {
          const clientVersion = info.getValue();
          return (
            <TableCell>
              {isPresent(clientVersion) && <span>{clientVersion}</span>}
              {!isPresent(clientVersion) && <UnknownBadge />}
            </TableCell>
          );
        },
      }),
      columnHelper.accessor('user_id', {
        id: 'user',
        header: m.devices_col_user(),
        enableSorting: true,
        size: 200,
        minSize: 150,
        cell: (info) => {
          return (
            <TableCell>
              <span>{usersDisplay?.[info.getValue()]}</span>
            </TableCell>
          );
        },
      }),
      columnHelper.accessor('connected', {
        header: m.devices_col_connection(),
        enableSorting: false,
        enableColumnFilter: true,
        meta: { filterOptions: filterOptions.connection },
        size: 140,
        minSize: 120,
        cell: (info) => {
          const connected = info.getValue();
          return (
            <TableCell>
              <ConnectionBadge connected={connected} removeBackground={!connected} />
            </TableCell>
          );
        },
      }),
      columnHelper.display({
        id: 'edit',
        header: '',
        enableSorting: false,
        size: tableEditColumnSize,
        enableResizing: false,
        cell: (info) => (
          <TableEditCell
            testId="device-edit"
            menuItems={getRowMenuItems(info.row.original)}
          />
        ),
      }),
    ],
    [getRowMenuItems, usersDisplay, filterOptions, isEnterprise],
  );

  const table = useReactTable({
    state: {
      rowSelection,
      sorting,
      columnFilters,
    },
    meta: {
      filterMessages: tableFilterMessages,
    },
    data: devices,
    columns,
    getRowId: (row) => String(row.id),
    enableRowSelection: true,
    onRowSelectionChange: setRowSelection,
    columnResizeMode: 'onChange',
    enableSorting: true,
    enableSortingRemoval: false,
    manualSorting: true,
    onSortingChange: setSorting,
    manualFiltering: true,
    onColumnFiltersChange: setColumnFilters,
    getCoreRowModel: getCoreRowModel(),
  });

  if (isLoading) return <TableSkeleton />;

  const isEmpty =
    devices.length === 0 && search.length === 0 && columnFilters.length === 0;
  if (isEmpty)
    return (
      <EmptyStateFlexible
        icon="devices"
        title={m.devices_empty_title()}
        subtitle={m.devices_empty_subtitle()}
      />
    );

  return (
    <>
      <TableTop text={m.devices_table_title()}>
        {selectedDevices.length > 0 && (
          <ButtonMenu
            variant="outlined"
            text={m.devices_bulk_actions()}
            iconRight="arrow-small"
            iconRightRotation="down"
            placement="bottom-start"
            testId="devices-bulk-actions"
            menuItems={bulkMenuItems}
          />
        )}
        <Search
          placeholder={m.devices_search_placeholder()}
          initialValue={search}
          onChange={setSearch}
        />
      </TableTop>
      <TableBody
        table={table}
        hasNextPage={hasNextPage}
        loadingNextPage={isFetchingNextPage}
        onNextPage={fetchNextPage}
      />
      {devices.length === 0 && (
        <EmptyStateFlexible
          icon="search"
          title={m.search_empty_common_title()}
          subtitle={m.search_empty_common_subtitle()}
        />
      )}
      <DeviceDetailsDrawer
        device={selectedDevice}
        onClose={() => setSelectedDeviceId(null)}
      />
    </>
  );
};
