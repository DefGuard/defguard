import { keepPreviousData, useInfiniteQuery, useQuery } from '@tanstack/react-query';
import {
  createColumnHelper,
  getCoreRowModel,
  type RowSelectionState,
  useReactTable,
} from '@tanstack/react-table';
import { useCallback, useMemo, useState } from 'react';
import { m } from '../../../../paraglide/messages';
import api from '../../../../shared/api/api';
import {
  DeviceAttestationStatus,
  type DeviceAttestationStatusValue,
  type DeviceOverview,
  DeviceState,
  DeviceVerification,
  type DeviceVerificationValue,
} from '../../../../shared/api/types';
import type { PolicyOsVariant } from '../../../../shared/components/SystemSelector/types';
import { TableSkeleton } from '../../../../shared/components/skeleton/TableSkeleton/TableSkeleton';
import { AppText } from '../../../../shared/defguard-ui/components/AppText/AppText';
import { Badge } from '../../../../shared/defguard-ui/components/Badge/Badge';
import {
  BadgeVariant,
  type BadgeVariantValue,
} from '../../../../shared/defguard-ui/components/Badge/types';
import { ButtonMenu } from '../../../../shared/defguard-ui/components/ButtonMenu/MenuButton';
import { EmptyStateFlexible } from '../../../../shared/defguard-ui/components/EmptyStateFlexible/EmptyStateFlexible';
import { Icon, type IconKindValue } from '../../../../shared/defguard-ui/components/Icon';
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
import { useDeviceActions } from '../../hooks/useDeviceActions';

const columnHelper = createColumnHelper<DeviceOverview>();

const osIconSize = 20;

const unknownBadge = <Badge variant={BadgeVariant.Neutral} text={m.devices_unknown()} />;

const mobileOsTypes: PolicyOsVariant[] = ['android', 'ios'];

const platformIcon = (osType: PolicyOsVariant): IconKindValue =>
  mobileOsTypes.includes(osType) ? 'mobile' : 'desktop';

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

export const DevicesOverviewTable = () => {
  const [search, setSearch] = useState('');

  const { data, fetchNextPage, isFetchingNextPage, isLoading } = useInfiniteQuery({
    queryKey: ['device', 'overview', { search }],
    initialPageParam: 1,
    queryFn: ({ pageParam }) =>
      api.device.getDevicesOverview({
        page: pageParam,
        search: search.length ? search : undefined,
      }),
    getNextPageParam: (lastPage) => lastPage.pagination.next_page,
    placeholderData: keepPreviousData,
  });

  const { data: usersDisplay } = useQuery(getUsersDisplayQueryOptions);

  const devices = useMemo(() => data?.pages.flatMap((page) => page.data) ?? [], [data]);
  const hasNextPage = isPresent(data?.pages.at(-1)?.pagination.next_page);

  const [rowSelection, setRowSelection] = useState<RowSelectionState>({});
  const clearSelection = useCallback(() => setRowSelection({}), []);
  const selectedDevices = useMemo(
    () => devices.filter((device) => rowSelection[String(device.id)]),
    [devices, rowSelection],
  );
  const getActionMenuItems = useDeviceActions();
  const bulkMenuItems = useMemo(
    () =>
      getActionMenuItems({
        devices: selectedDevices,
        testIdPrefix: 'devices-bulk',
        onSuccess: clearSelection,
      }),
    [getActionMenuItems, selectedDevices, clearSelection],
  );

  const columns = useMemo(
    () => [
      columnHelper.accessor('name', {
        header: m.form_label_device_name(),
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
      columnHelper.accessor('state', {
        header: m.devices_col_state(),
        size: 120,
        minSize: 100,
        cell: (info) => (
          <TableCell>
            {info.getValue() === DeviceState.Enabled ? (
              <AppText font={TextStyle.TBodySm400} color={ThemeVariable.FgMuted}>
                {m.state_enabled()}
              </AppText>
            ) : (
              <Badge
                variant={BadgeVariant.Critical}
                icon="status-locked"
                showIcon
                text={m.devices_state_blocked()}
              />
            )}
          </TableCell>
        ),
      }),
      columnHelper.accessor('attestation_status', {
        header: m.devices_col_attestation(),
        size: 170,
        minSize: 140,
        cell: (info) => {
          const status = info.getValue();
          if (status === DeviceAttestationStatus.NotAttested) return <TableCell />;
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
        size: 150,
        minSize: 120,
        cell: (info) => <TableCell>{verificationContent(info.getValue())}</TableCell>,
      }),
      columnHelper.accessor('os_type', {
        id: 'platform',
        header: m.devices_col_platform(),
        size: 100,
        minSize: 80,
        cell: (info) => {
          const osType = info.getValue();
          return (
            <TableCell>
              {isPresent(osType) ? (
                <Icon
                  icon={platformIcon(osType)}
                  size={osIconSize}
                  staticColor={ThemeVariable.FgNeutral}
                />
              ) : (
                unknownBadge
              )}
            </TableCell>
          );
        },
      }),
      columnHelper.accessor('os_type', {
        id: 'system',
        header: m.devices_col_system(),
        size: 200,
        minSize: 150,
        cell: (info) => {
          const osType = info.getValue();
          if (!isPresent(osType)) return <TableCell>{unknownBadge}</TableCell>;
          const osVersion = info.row.original.os_version;
          return (
            <TableCell>
              <Icon
                icon={policyOsVariantToIcon(osType)}
                size={osIconSize}
                staticColor={ThemeVariable.FgNeutral}
              />
              <span>
                {[policyOsVariantToText(osType), osVersion].filter(Boolean).join(' ')}
              </span>
            </TableCell>
          );
        },
      }),
      columnHelper.accessor('client_version', {
        header: m.devices_col_client_version(),
        size: 140,
        minSize: 120,
        cell: (info) => {
          const clientVersion = info.getValue();
          return (
            <TableCell>
              {isPresent(clientVersion) ? <span>{clientVersion}</span> : unknownBadge}
            </TableCell>
          );
        },
      }),
      columnHelper.accessor('user_id', {
        id: 'user',
        header: m.devices_col_user(),
        size: 200,
        minSize: 150,
        cell: (info) => {
          const userId = info.getValue();
          const display = isPresent(userId) ? usersDisplay?.[userId] : undefined;
          return (
            <TableCell>
              <span>{display}</span>
            </TableCell>
          );
        },
      }),
      columnHelper.accessor('connected', {
        header: m.devices_col_connection(),
        size: 140,
        minSize: 120,
        cell: (info) => {
          const connected = info.getValue();
          return (
            <TableCell>
              {connected ? (
                <Badge
                  variant={BadgeVariant.Success}
                  showIcon
                  text={m.devices_connection_online()}
                />
              ) : (
                <Badge
                  variant={BadgeVariant.Neutral}
                  showIcon
                  removeBackground
                  text={m.devices_connection_offline()}
                />
              )}
            </TableCell>
          );
        },
      }),
      columnHelper.display({
        id: 'edit',
        header: '',
        size: tableEditColumnSize,
        enableResizing: false,
        cell: (info) => (
          <TableEditCell
            testId="device-edit"
            menuItems={getActionMenuItems({
              devices: [info.row.original],
              testIdPrefix: 'device',
            })}
          />
        ),
      }),
    ],
    [getActionMenuItems, usersDisplay],
  );

  const table = useReactTable({
    state: {
      rowSelection,
    },
    data: devices,
    columns,
    getRowId: (row) => String(row.id),
    enableRowSelection: true,
    onRowSelectionChange: setRowSelection,
    columnResizeMode: 'onChange',
    enableSorting: false,
    manualFiltering: true,
    getCoreRowModel: getCoreRowModel(),
  });

  if (isLoading) return <TableSkeleton />;

  if (devices.length === 0 && search.length === 0)
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
    </>
  );
};
