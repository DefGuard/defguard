import { useMutation } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import {
  createColumnHelper,
  getCoreRowModel,
  useReactTable,
} from '@tanstack/react-table';
import { cloneDeep, flat } from 'radashi';
import { useCallback, useMemo, useState } from 'react';
import './RulesTable.scss';
import { m } from '../../paraglide/messages';
import api from '../../shared/api/api';
import {
  type AclAlias,
  type AclDestination,
  type AclRule,
  AclStatus,
  type AclStatusValue,
  type GroupInfo,
  type LicenseInfo,
  type NetworkDevice,
  type NetworkLocation,
  type ResourceById,
  type User,
} from '../../shared/api/types';
import { TableValuesListCell } from '../../shared/components/TableValuesListCell/TableValuesListCell';
import { Badge } from '../../shared/defguard-ui/components/Badge/Badge';
import { BadgeVariant } from '../../shared/defguard-ui/components/Badge/types';
import { Button } from '../../shared/defguard-ui/components/Button/Button';
import type { ButtonProps } from '../../shared/defguard-ui/components/Button/types';
import { EmptyStateFlexible } from '../../shared/defguard-ui/components/EmptyStateFlexible/EmptyStateFlexible';
import type {
  MenuItemProps,
  MenuItemsGroup,
} from '../../shared/defguard-ui/components/Menu/types';
import { Search } from '../../shared/defguard-ui/components/Search/Search';
import { tableEditColumnSize } from '../../shared/defguard-ui/components/table/consts';
import { TableBody } from '../../shared/defguard-ui/components/table/TableBody/TableBody';
import { TableCell } from '../../shared/defguard-ui/components/table/TableCell/TableCell';
import { TableEditCell } from '../../shared/defguard-ui/components/table/TableEditCell/TableEditCell';
import { TableTop } from '../../shared/defguard-ui/components/table/TableTop/TableTop';
import { Snackbar } from '../../shared/defguard-ui/providers/snackbar/snackbar';
import { isPresent } from '../../shared/defguard-ui/utils/isPresent';
import { canUseBusinessFeature, licenseActionCheck } from '../../shared/utils/license';

const displayUser = (user?: User): string => {
  if (!isPresent(user)) return '';

  if (user.first_name || user.last_name) {
    return `${user.first_name} ${user.last_name}`.trim();
  }
  return user.username;
};

type RowData = AclRule;

const columnHelper = createColumnHelper<RowData>();

type Props = {
  license: LicenseInfo | null;
  aliases: ResourceById<AclAlias>;
  destinations: ResourceById<AclDestination>;
  groups: ResourceById<GroupInfo>;
  users: ResourceById<User>;
  devices: ResourceById<NetworkDevice>;
  locations: ResourceById<NetworkLocation>;
  data: AclRule[];
  title: string;
  buttonProps: ButtonProps;
  variant: 'deployed' | 'pending';
  enableSearch?: boolean;
};

const toggleRulePromise = async (id: number) => {
  const rule = cloneDeep((await api.acl.rule.getRule(id)).data);
  rule.enabled = !rule.enabled;
  return api.acl.rule.editRule(rule);
};

export const RulesTable = ({
  title,
  buttonProps,
  enableSearch,
  aliases,
  destinations,
  devices,
  groups,
  users,
  locations,
  data,
  license,
  variant,
}: Props) => {
  const navigate = useNavigate();

  const { mutate: deleteRule } = useMutation({
    mutationFn: api.acl.rule.deleteRule,
    meta: {
      invalidate: ['acl'],
    },
  });

  const { mutate: toggleRule } = useMutation({
    mutationFn: toggleRulePromise,
    meta: {
      invalidate: ['acl'],
    },
  });

  const { mutate: deployRule } = useMutation({
    mutationFn: api.acl.rule.applyRules,
    onSuccess: () => {
      Snackbar.default(`Rule deployed`);
    },
    meta: {
      invalidate: ['acl'],
    },
  });

  const [search, setSearch] = useState('');

  const renderPermissionCell = useCallback(
    (
      permission: 'deny' | 'allow',
      permissionUsers: boolean,
      permissionGroup: boolean,
      permissionDevice: boolean,
      includedUsers: number[],
      includedGroups: number[],
      includedDevices: number[],
    ) => {
      if (permissionDevice && permissionGroup && permissionUsers) {
        return (
          <TableCell>
            {permission === 'allow' && (
              <Badge
                variant={BadgeVariant.Success}
                icon="check-filled"
                text="All allowed"
              />
            )}
            {permission === 'deny' && (
              <Badge
                variant={BadgeVariant.Warning}
                icon="status-important"
                text="All denied"
              />
            )}
          </TableCell>
        );
      }
      const display = flat([
        permissionUsers
          ? ['All users']
          : includedUsers.map((userId) => displayUser(users[userId])),
        permissionGroup
          ? ['All groups']
          : includedGroups.map((groupId) => groups[groupId]?.name ?? ''),
        permissionDevice
          ? ['All network devices']
          : includedDevices.map((deviceId) => devices[deviceId]?.name ?? ''),
      ]).filter((value) => value.length > 0);

      return <TableValuesListCell values={display} />;
    },
    [users, devices, groups],
  );

  const renderStatusCell = useCallback(
    (ruleState: AclStatusValue, isEnabled: boolean) => {
      // handle applied rules first
      if (ruleState === AclStatus.Applied) {
        return (
          <TableCell>
            {isEnabled ? (
              <Badge variant={BadgeVariant.Success} text="Active" />
            ) : (
              <Badge variant={BadgeVariant.Critical} text="Disabled" />
            )}
          </TableCell>
        );
      } else if (ruleState === AclStatus.Deleted) {
        return (
          <TableCell>
            <Badge variant={BadgeVariant.Critical} text="Deleted" />
          </TableCell>
        );
      } else {
        // handle remaining pending states
        return (
          <TableCell>
            <Badge variant={BadgeVariant.Warning} text={ruleState} />
          </TableCell>
        );
      }
    },
    [],
  );

  const columns = useMemo(
    () => [
      columnHelper.accessor('name', {
        header: 'Rule name',
        minSize: 210,
        meta: {
          flex: true,
        },
        cell: (info) => (
          <TableCell>
            <span>{info.getValue()}</span>
          </TableCell>
        ),
      }),
      columnHelper.display({
        id: 'destination',
        header: 'Destination',
        minSize: 350,
        cell: (info) => {
          const row = info.row.original;
          const manualAddresses = row.addresses.trim();
          const hasManualAddresses = manualAddresses.length > 0;

          return (
            <TableCell>
              {row.destinations.map((destinationId) => {
                const destination = destinations[destinationId];
                if (!destination) return null;
                return (
                  <Badge
                    className="rules-table-destination-badge"
                    data-marker="D"
                    variant={BadgeVariant.Neutral}
                    text={destination.name}
                    key={destinationId}
                  />
                );
              })}
              {hasManualAddresses && <span>{manualAddresses}</span>}
              {row.aliases.map((aliasId) => {
                const alias = aliases[aliasId];
                if (!alias) return null;

                return (
                  <Badge
                    className="rules-table-destination-badge"
                    data-marker="A"
                    variant={BadgeVariant.Neutral}
                    text={alias.name}
                    key={aliasId}
                  />
                );
              })}
            </TableCell>
          );
        },
      }),
      columnHelper.display({
        id: 'permissions',
        header: 'Permissions',
        minSize: 220,
        cell: (info) => {
          const row = info.row.original;
          return renderPermissionCell(
            'allow',
            row.allow_all_users,
            row.allow_all_groups,
            row.allow_all_network_devices,
            row.allowed_users,
            row.allowed_groups,
            row.allowed_network_devices,
          );
        },
      }),
      columnHelper.display({
        id: 'restrictions',
        header: 'Restrictions',
        minSize: 220,
        cell: (info) => {
          const row = info.row.original;
          return renderPermissionCell(
            'deny',
            row.deny_all_users,
            row.deny_all_groups,
            row.deny_all_network_devices,
            row.denied_users,
            row.denied_groups,
            row.denied_network_devices,
          );
        },
      }),
      columnHelper.display({
        id: 'locations',
        header: 'Locations',
        minSize: 220,
        cell: (info) => {
          const row = info.row.original;
          if (row.all_locations) {
            return (
              <TableCell>
                <Badge
                  variant={BadgeVariant.Success}
                  text="All locations"
                  icon="check-filled"
                />
              </TableCell>
            );
          }
          const locationNames = row.locations
            .map((locationId) => locations[locationId]?.name ?? '')
            .filter((name) => name.length);

          return <TableValuesListCell values={locationNames} />;
        },
      }),
      columnHelper.display({
        id: 'status',
        header: 'Status',
        size: 125,
        minSize: 125,
        cell: (info) => {
          const row = info.row.original;
          return renderStatusCell(row.state, row.enabled);
        },
      }),
      columnHelper.display({
        id: 'edit',
        header: '',
        size: tableEditColumnSize,
        enableResizing: false,
        cell: (info) => {
          const row = info.row.original;
          const topItems: MenuItemProps[] = [
            {
              icon: 'edit',
              text: m.controls_edit(),
              onClick: () => {
                licenseActionCheck(canUseBusinessFeature(license), () => {
                  navigate({
                    to: '/acl/edit-rule',
                    search: {
                      rule: row.id,
                    },
                  });
                });
              },
            },
          ];
          switch (variant) {
            case 'deployed':
              if (row.enabled) {
                topItems.push({
                  icon: 'disabled',
                  text: m.controls_disable(),
                  onClick: () => {
                    toggleRule(row.id);
                  },
                });
              } else {
                topItems.push({
                  icon: 'check',
                  text: m.controls_enable(),
                  onClick: () => {
                    toggleRule(row.id);
                  },
                });
              }
              break;
            case 'pending':
              topItems.push({
                icon: 'deploy',
                text: m.controls_deploy(),
                onClick: () => {
                  deployRule([row.id]);
                },
              });
              break;
          }
          const menuItems: MenuItemsGroup[] = [
            {
              items: topItems,
            },
            {
              items: [
                {
                  icon: 'delete',
                  variant: 'danger',
                  text: m.controls_delete(),
                  onClick: () => {
                    licenseActionCheck(canUseBusinessFeature(license), () => {
                      deleteRule(row.id);
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
    [
      aliases,
      destinations,
      renderPermissionCell,
      deleteRule,
      locations,
      navigate,
      renderStatusCell,
      license,
      variant,
      toggleRule,
      deployRule,
    ],
  );

  const visibleRules = useMemo(() => {
    let res = data;
    if (search.length) {
      res = res.filter((rule) => rule.name.toLowerCase().includes(search.toLowerCase()));
    }
    return res;
  }, [search, data]);

  const table = useReactTable({
    columns,
    data: visibleRules,
    enableRowSelection: false,
    columnResizeMode: 'onChange',
    getCoreRowModel: getCoreRowModel(),
  });

  if (data.length === 0) return null;

  return (
    <>
      <TableTop text={title}>
        {enableSearch && (
          <Search placeholder={m.controls_search()} value={search} onChange={setSearch} />
        )}
        <Button {...buttonProps} />
      </TableTop>
      {visibleRules.length > 0 && <TableBody table={table} />}
      {visibleRules.length === 0 && (
        <EmptyStateFlexible
          icon="search"
          title={m.search_empty_common_title()}
          subtitle={m.search_empty_common_subtitle()}
        />
      )}
    </>
  );
};
